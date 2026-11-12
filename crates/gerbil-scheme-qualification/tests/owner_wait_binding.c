/* Exercise the actual wrapper in a private image, not a replacement select. */
#include "../ffi/owner_wait_scope.h"
#include <dlfcn.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/select.h>
#include <sys/time.h>
#include <unistd.h>

#define REQUIRE(test) do { if (!(test)) { \
  fprintf(stderr, "OWNER-WAIT-BINDING-FAILED line %d: %s (errno %d)\n", __LINE__, #test, errno); return 1; \
} } while (0)
struct api {
  int (*open)(struct owner_wait_scope_handle *);
  int (*progress)(struct owner_wait_scope_handle, uint64_t, uint64_t);
  int (*finish)(struct owner_wait_scope_handle, int, struct owner_wait_scope_snapshot *);
  uint64_t (*clock)(void);
  int (*select)(int, fd_set *, fd_set *, fd_set *, struct timeval *);
  int (*original_distinct)(void);
  void (*on_enter)(void (*)(void *), void *);
  int (*exhaustion_contract)(void);
  uint64_t (*legacy_recording)(int);
};

static volatile sig_atomic_t interrupted;
static void interrupt_select(int signal_number) { (void)signal_number; interrupted = 1; }

static int real_wait_outcomes(struct api *api, uint64_t *generation) {
  int pipefd[2], dead[2];
  REQUIRE(pipe(pipefd) == 0 && pipe(dead) == 0);
  REQUIRE(close(dead[0]) == 0 && close(dead[1]) == 0);
  struct owner_wait_scope_handle handle;
  struct owner_wait_scope_snapshot snapshot;
  REQUIRE(api->legacy_recording(1) == 0);
  REQUIRE(api->open(&handle) == EBUSY);
  REQUIRE(api->legacy_recording(0) == 0);
  errno = EDOM;
  REQUIRE(api->open(&handle) == 0 && errno == EDOM);
  REQUIRE(api->legacy_recording(1) == EBUSY && errno == EDOM);
  struct owner_wait_scope_handle untouched = {123, 456};
  REQUIRE(api->open(&untouched) == EBUSY && untouched.generation == 123 && untouched.owner == 456);
  REQUIRE(api->finish(handle, 0, NULL) == EINVAL);
  uint64_t previous = api->clock();
  REQUIRE(api->progress(handle, 0, previous) == 0 && errno == EDOM);
  REQUIRE(api->original_distinct());
  REQUIRE(write(pipefd[1], "x", 1) == 1);
  fd_set readers;
  FD_ZERO(&readers); FD_SET(pipefd[0], &readers);
  errno = EDOM;
  REQUIRE(api->select(pipefd[0]+1, &readers, NULL, NULL, NULL) == 1);
  REQUIRE(errno == EDOM && FD_ISSET(pipefd[0], &readers));
  char byte = 0;
  REQUIRE(read(pipefd[0], &byte, 1) == 1 && byte == 'x'); /* wrapper did not consume it */
  struct timeval zero = {0, 0};
  errno = ERANGE;
  REQUIRE(api->select(0, NULL, NULL, NULL, &zero) == 0 && errno == ERANGE);
  FD_ZERO(&readers); FD_SET(dead[0], &readers);
  struct timeval finite = {0, 100000};
  errno = 0;
  REQUIRE(api->select(dead[0]+1, &readers, NULL, NULL, &finite) == -1 && errno == EBADF);
  struct sigaction action = {0}, old_action;
  action.sa_handler = interrupt_select;
  sigemptyset(&action.sa_mask);
  REQUIRE(sigaction(SIGALRM, &action, &old_action) == 0);
  struct itimerval timer = { .it_interval = {0, 10000}, .it_value = {0, 10000} };
  REQUIRE(setitimer(ITIMER_REAL, &timer, NULL) == 0);
  FD_ZERO(&readers); FD_SET(pipefd[0], &readers);
  errno = 0;
  int result = api->select(pipefd[0]+1, &readers, NULL, NULL, NULL);
  int call_errno = errno;
  struct itimerval off = {0};
  REQUIRE(setitimer(ITIMER_REAL, &off, NULL) == 0);
  REQUIRE(sigaction(SIGALRM, &old_action, NULL) == 0);
  REQUIRE(result == -1 && call_errno == EINTR && interrupted);
  uint64_t now = api->clock();
  errno = EDOM;
  REQUIRE(api->progress(handle, previous, now) == 0 && errno == EDOM);
  REQUIRE(api->finish(handle, 0, &snapshot) == 0 && errno == EDOM);
  REQUIRE(api->finish(handle, 0, &snapshot) == 0); /* cold close is idempotent */
  *generation = handle.generation;
  REQUIRE(owner_wait_scope_complete_gap(&snapshot));
  REQUIRE(snapshot.gap.calls == 4 && snapshot.gap.ready == 1 && snapshot.gap.timed_out == 1);
  REQUIRE(snapshot.gap.interrupted == 1 && snapshot.gap.failed == 1);
  REQUIRE(snapshot.gap.zero_timeout == 1 && snapshot.gap.finite_timeout == 1 && snapshot.gap.infinite_timeout == 2);
  REQUIRE(snapshot.gap.call_wall <= snapshot.gap.end - snapshot.gap.begin);
  REQUIRE(close(pipefd[0]) == 0 && close(pipefd[1]) == 0);
  return 0;
}

struct cancellation {
  struct api *api;
  int descriptor;
  pthread_mutex_t lock;
  pthread_cond_t condition;
  int entered, finish_status;
  struct owner_wait_scope_handle handle;
  struct owner_wait_scope_snapshot snapshot;
};
static void entered(void *argument) {
  struct cancellation *c = argument;
  pthread_mutex_lock(&c->lock);
  c->entered = 1;
  pthread_cond_signal(&c->condition);
  pthread_mutex_unlock(&c->lock);
}
static void outer_cleanup(void *argument) {
  struct cancellation *c = argument;
  /* Only the wrapper's inner cancellation cleanup may abort/retire its call.
   * An outer forced abort here would mask missing wrapper cleanup. */
  c->finish_status = c->api->finish(c->handle, 0, &c->snapshot);
}
static void *cancelled_owner(void *argument) {
  struct cancellation *c = argument;
  int status = c->api->open(&c->handle);
  if (status) return (void *)(uintptr_t)1;
  uint64_t now = c->api->clock();
  if (c->api->progress(c->handle, 0, now)) return (void *)(uintptr_t)2;
  fd_set readers;
  FD_ZERO(&readers); FD_SET(c->descriptor, &readers);
  pthread_cleanup_push(outer_cleanup, c);
  (void)c->api->select(c->descriptor+1, &readers, NULL, NULL, NULL);
  pthread_cleanup_pop(1);
  return (void *)(uintptr_t)3;
}
struct reuse {
  struct api *api;
  struct owner_wait_scope_handle stale;
  uint64_t new_generation;
  int passed;
};
static void *next_owner(void *argument) {
  struct reuse *r = argument;
  struct owner_wait_scope_handle handle;
  struct owner_wait_scope_snapshot snapshot;
  if (r->api->open(&handle)) return NULL;
  r->new_generation = handle.generation;
  uint64_t now = r->api->clock();
  if (r->api->progress(r->stale, 0, now) != EPERM ||
      r->api->finish(r->stale, 1, &snapshot) != EPERM) return NULL;
  if (r->api->progress(handle, 0, now) || r->api->finish(handle, 0, &snapshot)) return NULL;
  r->passed = !snapshot.aborted && handle.generation > r->stale.generation;
  return NULL;
}
static int cancellation_and_affinity(struct api *api, uint64_t previous_generation) {
  int pipefd[2];
  REQUIRE(pipe(pipefd) == 0);
  struct cancellation c = { .api=api, .descriptor=pipefd[0],
    .lock=PTHREAD_MUTEX_INITIALIZER, .condition=PTHREAD_COND_INITIALIZER, .finish_status=-1 };
  api->on_enter(entered, &c);
  pthread_t worker;
  REQUIRE(pthread_create(&worker, NULL, cancelled_owner, &c) == 0);
  REQUIRE(pthread_mutex_lock(&c.lock) == 0);
  while (!c.entered) REQUIRE(pthread_cond_wait(&c.condition, &c.lock) == 0);
  REQUIRE(pthread_mutex_unlock(&c.lock) == 0);
  struct owner_wait_scope_snapshot wrong_thread;
  REQUIRE(api->finish(c.handle, 1, &wrong_thread) == EPERM);
  REQUIRE(pthread_cancel(worker) == 0);
  void *result = NULL;
  REQUIRE(pthread_join(worker, &result) == 0 && result == PTHREAD_CANCELED);
  api->on_enter(NULL, NULL);
  REQUIRE(c.finish_status == 0 && c.snapshot.aborted && c.snapshot.invalid);
  REQUIRE(c.handle.generation > previous_generation);
  REQUIRE(!owner_wait_scope_complete_gap(&c.snapshot));
  REQUIRE(pthread_cond_destroy(&c.condition) == 0 && pthread_mutex_destroy(&c.lock) == 0);
  REQUIRE(close(pipefd[0]) == 0 && close(pipefd[1]) == 0);
  struct reuse reuse = { .api=api, .stale=c.handle };
  REQUIRE(pthread_create(&worker, NULL, next_owner, &reuse) == 0);
  REQUIRE(pthread_join(worker, NULL) == 0);
  REQUIRE(reuse.passed && reuse.new_generation > c.handle.generation);
  return 0;
}

int main(int argc, char **argv) {
  REQUIRE(argc == 2);
  void *image = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
  if (!image) { fprintf(stderr, "%s\n", dlerror()); return 1; }
  struct api api;
#define LOAD(field, name) do { \
  void *symbol = dlsym(image, name); REQUIRE(symbol != NULL); \
  _Static_assert(sizeof(symbol) == sizeof(api.field), "POSIX function pointer size"); \
  memcpy(&api.field, &symbol, sizeof(symbol)); \
} while (0)
  LOAD(open, "gws_open"); LOAD(progress, "gws_progress"); LOAD(finish, "gws_finish");
  LOAD(clock, "gws_clock"); LOAD(select, "gws_select");
  LOAD(original_distinct, "gws_original_distinct"); LOAD(on_enter, "gws_on_enter");
  LOAD(exhaustion_contract, "gws_exhaustion_contract");
  LOAD(legacy_recording, "gws_legacy_recording");
  uint64_t previous_generation = 0;
  if (real_wait_outcomes(&api, &previous_generation) ||
      cancellation_and_affinity(&api, previous_generation)) return 1;
  REQUIRE(api.exhaustion_contract());
  REQUIRE(dlclose(image) == 0);
  puts("OWNER-WAIT-BINDING-CONTRACT-OK");
  return 0;
}
