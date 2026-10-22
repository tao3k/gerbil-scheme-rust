/* Qualification-only observation of this image's native select calls. */
#include <sys/select.h>
#include <dlfcn.h>
#include <pthread.h>
#include <pthread/qos.h>
#include <errno.h>
#include <sys/stat.h>
#include <sys/ioctl.h>
#define OWNER_WAIT_ROWS 4096
static struct {
  uint64_t rows[OWNER_WAIT_ROWS][6];
  uint64_t count, overflow, errors;
} owner_wait;
static _Thread_local int owner_wait_enabled;
static uint64_t owner_wait_anchor[3];
/* Qualification-only bridge between native monotonic time and kernel trace time.
 * Create/release one log per observation reset; no retained logging resource. */
static void owner_wait_mark(void);
extern void gerbil_owner_clock_signpost(uint64_t monotonic);
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
  clock_gettime(CLOCK_UPTIME_RAW, &now);
  return (uint64_t)now.tv_sec * 1000000000ULL + (uint64_t)now.tv_nsec;
}
static void owner_wait_mark(void) {
  pthread_threadid_np(NULL, &owner_wait_anchor[2]);
  owner_wait_anchor[0] = owner_wait_clock();
  gerbil_owner_clock_signpost(owner_wait_anchor[0]);
  owner_wait_anchor[1] = owner_wait_clock();
}
static uint64_t owner_wait_cpu(void) {
  struct timespec now;
  clock_gettime(CLOCK_THREAD_CPUTIME_ID, &now);
  return (uint64_t)now.tv_sec * 1000000000ULL + (uint64_t)now.tv_nsec;
}
static void owner_wait_record(uint64_t kind, uint64_t start, uint64_t end, uint64_t requested, uint64_t cpu, uint64_t flags) {
  if (owner_wait.count == OWNER_WAIT_ROWS) { owner_wait.overflow++; return; }
  uint64_t *row = owner_wait.rows[owner_wait.count++];
  row[0] = kind; row[1] = start; row[2] = end; row[3] = requested; row[4] = cpu; row[5] = flags;
}
/* Register the original read endpoint; Scheme opens a duplicate of this pipe.
 * Observation identifies that duplicate without consuming any wake byte. */
static struct stat owner_wait_pipe;
static int owner_wait_pipe_known, owner_wait_pipe_fd = -1;
static int owner_wait_descriptor(int count, fd_set *readers) {
  if (!readers || !owner_wait_pipe_known) return -1;
  if (owner_wait_pipe_fd >= 0 && owner_wait_pipe_fd < count && FD_ISSET(owner_wait_pipe_fd, readers))
    return owner_wait_pipe_fd;
  for (int fd = 0; fd < count; ++fd) {
    struct stat candidate;
    if (FD_ISSET(fd, readers) && fstat(fd, &candidate) == 0 &&
        S_ISFIFO(candidate.st_mode) && candidate.st_dev == owner_wait_pipe.st_dev &&
        candidate.st_ino == owner_wait_pipe.st_ino) {
      owner_wait_pipe_fd = fd; return fd;
    }
  }
  return -1;
}
static int owner_wait_readable(int fd) {
  int count = 0;
  return fd >= 0 && ioctl(fd, FIONREAD, &count) == 0 && count > 0;
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
  uint64_t requested = timeout ? (uint64_t)timeout->tv_sec * 1000000000ULL + (uint64_t)timeout->tv_usec * 1000ULL : INT64_MAX;
  int wake_fd = observed ? owner_wait_descriptor(count, readers) : -1;
  uint64_t flags = wake_fd >= 0 ? 1 : 0;
  if (observed && owner_wait_readable(wake_fd)) flags |= 2;
  uint64_t cpu = observed ? owner_wait_cpu() : 0;
  uint64_t start = observed ? owner_wait_clock() : 0;
  errno = incoming_errno;
  int result = owner_select_original(count, readers, writers, errors, timeout);
  int saved_errno = errno;
  if (observed) {
    uint64_t kind = result == 0 ? 1 : result > 0 ? 4 : saved_errno == EINTR ? 3 : 5;
    uint64_t end = owner_wait_clock(), used = owner_wait_cpu() - cpu;
    if (wake_fd >= 0 && result > 0 && FD_ISSET(wake_fd, readers)) flags |= 4;
    if (owner_wait_readable(wake_fd)) flags |= 8;
    owner_wait_record(kind, start, end, requested, used, flags);
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
      if (owner_wait_enabled) owner_wait_mark();
      return 0;
    case 8:
      owner_wait_pipe_known = fstat((int)value, &owner_wait_pipe) == 0 && S_ISFIFO(owner_wait_pipe.st_mode);
      owner_wait_pipe_fd = -1;
      return owner_wait_pipe_known ? 0 : UINT64_MAX;
    case 9: return value < 3 ? owner_wait_anchor[value] : UINT64_MAX;
    case 10:
      owner_wait_enabled = 0;
      owner_wait.count = owner_wait.overflow = owner_wait.errors = 0;
      owner_wait_mark();
      return 0;
    case 5: return owner_wait_qos_set(value);
    case 6: return (uint64_t)qos_class_self();
    case 7: return (uint64_t)owner_wait_qos;
    case 1: return owner_wait_clock();
    case 2: owner_wait_record(2, value, owner_wait_clock(), 1000000, 0, 0); return 0;
    case 3: owner_wait_enabled = 0; return owner_wait.count * 6 + 2;
    case 4:
      if (value == 0) return owner_wait.overflow;
      if (value == 1) return owner_wait.errors;
      value -= 2;
      return value < owner_wait.count * 6 ? owner_wait.rows[value / 6][value % 6] : INT64_MAX;
    default: return UINT64_MAX;
  }
}

extern void gerbil_owner_wait_bind(uint64_t (*observe)(int, uint64_t));
__attribute__((constructor)) static void owner_wait_bind(void) {
  gerbil_owner_wait_bind(gerbil_owner_wait_observe);
}
