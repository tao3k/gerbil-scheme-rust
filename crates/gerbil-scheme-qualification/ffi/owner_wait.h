/* Qualification-only observation of this image's native select calls. */
#include <sys/select.h>
#include <dlfcn.h>
#include <pthread.h>
#include <pthread/qos.h>
#include <errno.h>
#define OWNER_WAIT_ROWS 4096
static struct {
  uint64_t rows[OWNER_WAIT_ROWS][4];
  uint64_t count, overflow, errors;
} owner_wait;
static _Thread_local int owner_wait_enabled;
static qos_class_t owner_wait_qos;
static int owner_wait_relative, owner_wait_qos_saved;
static uint64_t owner_wait_qos_set(uint64_t elevated) {
  qos_class_t current;
  int relative;
  int status = pthread_get_qos_class_np(pthread_self(), &current, &relative);
  if (status) return (uint64_t)status;
  if (!owner_wait_qos_saved) {
    if (current == QOS_CLASS_UNSPECIFIED) return EINVAL;
    owner_wait_qos = current; owner_wait_relative = relative;
    owner_wait_qos_saved = 1;
  }
  return (uint64_t)pthread_set_qos_class_self_np(
    elevated ? QOS_CLASS_USER_INITIATED : owner_wait_qos,
    elevated ? 0 : owner_wait_relative);
}
static uint64_t owner_wait_clock(void) {
  struct timespec now;
  clock_gettime(CLOCK_MONOTONIC, &now);
  return (uint64_t)now.tv_sec * 1000000000ULL + (uint64_t)now.tv_nsec;
}
static void owner_wait_record(uint64_t kind, uint64_t start, uint64_t end, uint64_t requested) {
  if (owner_wait.count == OWNER_WAIT_ROWS) { owner_wait.overflow++; return; }
  uint64_t *row = owner_wait.rows[owner_wait.count++];
  row[0] = kind; row[1] = start; row[2] = end; row[3] = requested;
}
typedef int (*owner_select_fn)(int, fd_set *, fd_set *, fd_set *, struct timeval *);
static owner_select_fn owner_select_original;
static pthread_once_t owner_select_once = PTHREAD_ONCE_INIT;
static void owner_select_resolve(void) {
  owner_select_original = (owner_select_fn)dlsym(RTLD_NEXT, "select");
}
/* Hidden by the image export list: only the embedded SDK binds this symbol.
 * Forward every argument and preserve errno; do not change SDK timeout policy. */
int select(int count, fd_set *readers, fd_set *writers, fd_set *errors, struct timeval *timeout) {
  int incoming_errno = errno;
  pthread_once(&owner_select_once, owner_select_resolve);
  errno = incoming_errno;
  if (!owner_select_original) { errno = ENOSYS; return -1; }
  int observed = owner_wait_enabled;
  uint64_t start = observed ? owner_wait_clock() : 0;
  uint64_t requested = timeout ? (uint64_t)timeout->tv_sec * 1000000000ULL + (uint64_t)timeout->tv_usec * 1000ULL : INT64_MAX;
  int result = owner_select_original(count, readers, writers, errors, timeout);
  int saved_errno = errno;
  if (observed) {
    uint64_t kind = result == 0 ? 1 : result > 0 ? 4 : saved_errno == EINTR ? 3 : 5;
    owner_wait_record(kind, start, owner_wait_clock(), requested);
    if (result < 0 && saved_errno != EINTR) owner_wait.errors++;
  }
  errno = saved_errno;
  return result;
}
static uint64_t gerbil_owner_wait_observe(int operation, uint64_t value) {
  switch (operation) {
    case 0:
      owner_wait_enabled = 0;
      owner_wait.count = owner_wait.overflow = owner_wait.errors = 0;
      owner_wait_enabled = value != 0;
      return 0;
    case 5: return owner_wait_qos_set(value);
    case 6: return (uint64_t)qos_class_self();
    case 7: return (uint64_t)owner_wait_qos;
    case 1: return owner_wait_clock();
    case 2: owner_wait_record(2, value, owner_wait_clock(), 1000000); return 0;
    case 3: owner_wait_enabled = 0; return owner_wait.count * 4 + 2;
    case 4:
      if (value == 0) return owner_wait.overflow;
      if (value == 1) return owner_wait.errors;
      value -= 2;
      return value < owner_wait.count * 4 ? owner_wait.rows[value / 4][value % 4] : INT64_MAX;
    default: return UINT64_MAX;
  }
}

extern void gerbil_owner_wait_bind(uint64_t (*observe)(int, uint64_t));
__attribute__((constructor)) static void owner_wait_bind(void) {
  gerbil_owner_wait_bind(gerbil_owner_wait_observe);
}
