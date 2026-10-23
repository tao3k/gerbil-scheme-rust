/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
/* The paired Darwin SDK uses process-wide ITIMER_REAL/SIGALRM. Only the
 * statically linked SDK in this private image resolves these hidden symbols;
 * host libc callers keep their original timer and signal disposition.
 * One joinable OS notifier invokes the SDK interrupt primitive, never Scheme
 * or GC entry. The VM retains exactly one execution/collector owner thread. */
#include <pthread.h>
#include <signal.h>
#include <sys/time.h>
#include <time.h>
#include <limits.h>
static pthread_mutex_t gerbil_timer_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t gerbil_timer_changed = PTHREAD_COND_INITIALIZER;
static struct sigaction gerbil_timer_action;
static int gerbil_timer_action_set;
static int gerbil_timer_action_registered;
static pthread_t gerbil_timer_owner;
static pthread_t gerbil_timer_thread;
static int gerbil_timer_started;
static uint64_t gerbil_timer_deadline;
static uint64_t gerbil_timer_period;
static int gerbil_timer_closed;
static int gerbil_timer_failure;
static int gerbil_timer_masked;
static int gerbil_timer_pending;
static void gerbil_timer_init_owner(void) { gerbil_timer_owner = pthread_self(); }
static uint64_t gerbil_timer_now(void) {
  struct timespec value;
  if (clock_gettime(CLOCK_MONOTONIC, &value) != 0) {
    gerbil_timer_failure = errno;
    return 0;
  }
  return (uint64_t)value.tv_sec * 1000000000u + (uint64_t)value.tv_nsec;
}
static int gerbil_timer_duration(struct timeval value, uint64_t *out) {
  if (value.tv_sec < 0 || value.tv_usec < 0 || value.tv_usec >= 1000000 ||
      (uint64_t)value.tv_sec > ((uint64_t)INT64_MAX - 999999000u) / 1000000000u) {
    errno = EINVAL;
    return -1;
  }
  *out = (uint64_t)value.tv_sec * 1000000000u + (uint64_t)value.tv_usec * 1000u;
  return 0;
}
static struct timeval gerbil_timer_timeval(uint64_t value) {
  uint64_t micros = (value + 999u) / 1000u;
  return (struct timeval){ (time_t)(micros / 1000000u), (suseconds_t)(micros % 1000000u) };
}
static void gerbil_timer_read(struct itimerval *out) {
  uint64_t now = gerbil_timer_now();
  uint64_t remaining = gerbil_timer_deadline > now ? gerbil_timer_deadline - now : 0;
  *out = (struct itimerval){ gerbil_timer_timeval(gerbil_timer_period), gerbil_timer_timeval(remaining) };
}
static void gerbil_timer_deliver(void) {
  if (!gerbil_timer_action_registered) return;
  void (*handler)(int) = gerbil_timer_action.sa_handler;
  if (handler != SIG_IGN && handler != SIG_DFL && handler != NULL) handler(SIGALRM);
}
static void *gerbil_timer_run(void *unused) {
  (void)unused;
  pthread_mutex_lock(&gerbil_timer_lock);
  while (!gerbil_timer_closed) {
    if (gerbil_timer_deadline == 0 || gerbil_timer_failure != 0) {
      int status = pthread_cond_wait(&gerbil_timer_changed, &gerbil_timer_lock);
      if (status != 0) { gerbil_timer_failure = status; break; }
      continue;
    }
    uint64_t now = gerbil_timer_now();
    if (gerbil_timer_failure != 0) continue;
    if (now < gerbil_timer_deadline) {
      uint64_t delay = gerbil_timer_deadline - now;
      struct timespec relative = { (time_t)(delay / 1000000000u), (long)(delay % 1000000000u) };
      int status = pthread_cond_timedwait_relative_np(&gerbil_timer_changed, &gerbil_timer_lock, &relative);
      if (status != 0 && status != ETIMEDOUT && status != EINTR) gerbil_timer_failure = status;
      continue;
    }
    if (gerbil_timer_period != 0) {
      uint64_t missed = (now - gerbil_timer_deadline) / gerbil_timer_period + 1;
      gerbil_timer_deadline += missed * gerbil_timer_period;
    } else gerbil_timer_deadline = 0;
    /* The fixed SDK's sigaction handler only raises ___INTR_HEARTBEAT.
     * Serialize notification with the owner's native heartbeat mask. */
    if (gerbil_timer_masked) gerbil_timer_pending = 1;
    else gerbil_timer_deliver();
  }
  pthread_mutex_unlock(&gerbil_timer_lock);
  return NULL;
}
int sigaction(int signal_number, const struct sigaction *action, struct sigaction *previous) {
  typedef int (*real_action)(int, const struct sigaction *, struct sigaction *);
  real_action system_action = (real_action)dlsym(RTLD_DEFAULT, "sigaction");
  if (system_action == NULL || system_action == sigaction) { errno = ENOSYS; return -1; }
  if (signal_number != SIGALRM) return system_action(signal_number, action, previous);
  /* This ABI is deliberately the paired SDK's simple handler, not SA_SIGINFO. */
  if (action != NULL && (action->sa_flags & (SA_SIGINFO | SA_RESETHAND | SA_NODEFER))) {
    errno = ENOTSUP;
    return -1;
  }
  pthread_mutex_lock(&gerbil_timer_lock);
  if (!gerbil_timer_action_set) {
    if (system_action(SIGALRM, NULL, &gerbil_timer_action) != 0) {
      pthread_mutex_unlock(&gerbil_timer_lock);
      return -1;
    }
    gerbil_timer_action_set = 1;
  }
  if (previous != NULL) *previous = gerbil_timer_action;
  if (action != NULL) {
    gerbil_timer_action = *action;
    gerbil_timer_action_registered = 1;
  }
  pthread_mutex_unlock(&gerbil_timer_lock);
  return 0;
}
int pthread_sigmask(int how, const sigset_t *set, sigset_t *previous) {
  typedef int (*real_mask)(int, const sigset_t *, sigset_t *);
  real_mask system_mask = (real_mask)dlsym(RTLD_DEFAULT, "pthread_sigmask");
  if (system_mask == NULL || system_mask == pthread_sigmask) return ENOSYS;
  int status = system_mask(how, set, previous);
  if (status != 0 || set == NULL || !pthread_equal(pthread_self(), gerbil_timer_owner)) return status;
  int contains = sigismember(set, SIGALRM);
  pthread_mutex_lock(&gerbil_timer_lock);
  if (how == SIG_SETMASK) gerbil_timer_masked = contains == 1;
  else if (contains == 1 && how == SIG_BLOCK) gerbil_timer_masked = 1;
  else if (contains == 1 && how == SIG_UNBLOCK) gerbil_timer_masked = 0;
  if (!gerbil_timer_masked && gerbil_timer_pending && !gerbil_timer_closed) {
    gerbil_timer_pending = 0;
    gerbil_timer_deliver();
  }
  pthread_mutex_unlock(&gerbil_timer_lock);
  return status;
}
int getitimer(int which, struct itimerval *value) {
  typedef int (*real_get)(int, struct itimerval *);
  if (which != ITIMER_REAL) {
    real_get system_get = (real_get)dlsym(RTLD_DEFAULT, "getitimer");
    if (system_get == NULL || system_get == getitimer) { errno = ENOSYS; return -1; }
    return system_get(which, value);
  }
  if (value == NULL) { errno = EFAULT; return -1; }
  pthread_mutex_lock(&gerbil_timer_lock);
  gerbil_timer_read(value);
  pthread_mutex_unlock(&gerbil_timer_lock);
  return 0;
}
int setitimer(int which, const struct itimerval *value, struct itimerval *previous) {
  typedef int (*real_set)(int, const struct itimerval *, struct itimerval *);
  if (which != ITIMER_REAL) {
    real_set system_set = (real_set)dlsym(RTLD_DEFAULT, "setitimer");
    if (system_set == NULL || system_set == setitimer) { errno = ENOSYS; return -1; }
    return system_set(which, value, previous);
  }
  uint64_t first, period;
  if (value == NULL) { errno = EFAULT; return -1; }
  if (gerbil_timer_duration(value->it_value, &first) != 0 ||
      gerbil_timer_duration(value->it_interval, &period) != 0) return -1;
  pthread_mutex_lock(&gerbil_timer_lock);
  if (previous != NULL) gerbil_timer_read(previous);
  if (gerbil_timer_closed && first != 0) {
    pthread_mutex_unlock(&gerbil_timer_lock);
    errno = EINVAL;
    return -1;
  }
  if (first != 0 && !gerbil_timer_started) {
    int status = pthread_create(&gerbil_timer_thread, NULL, gerbil_timer_run, NULL);
    if (status != 0) {
      gerbil_timer_failure = status;
      pthread_mutex_unlock(&gerbil_timer_lock);
      errno = status;
      return -1;
    }
    gerbil_timer_started = 1;
  }
  uint64_t now = gerbil_timer_now();
  gerbil_timer_deadline = first == 0 ? 0 : now + first;
  gerbil_timer_period = period;
  pthread_cond_signal(&gerbil_timer_changed);
  pthread_mutex_unlock(&gerbil_timer_lock);
  return 0;
}
static int gerbil_timer_status(void) {
  pthread_mutex_lock(&gerbil_timer_lock);
  int status = gerbil_timer_failure;
  if (status == 0 && gerbil_timer_started && !gerbil_timer_action_registered) status = ENOTSUP;
  pthread_mutex_unlock(&gerbil_timer_lock);
  return status;
}
static int gerbil_timer_cleanup(void) {
  pthread_mutex_lock(&gerbil_timer_lock);
  gerbil_timer_closed = 1;
  gerbil_timer_deadline = 0;
  int started = gerbil_timer_started;
  pthread_t thread = gerbil_timer_thread;
  pthread_cond_signal(&gerbil_timer_changed);
  pthread_mutex_unlock(&gerbil_timer_lock);
  /* pthread_join covers the notifier's complete native return, not merely a
   * signaled completion event. No timer callback or code frame outlives it. */
  int status = started ? pthread_join(thread, NULL) : 0;
  if (status == 0) {
    pthread_mutex_lock(&gerbil_timer_lock);
    gerbil_timer_started = 0;
    pthread_mutex_unlock(&gerbil_timer_lock);
  }
  return status;
}
