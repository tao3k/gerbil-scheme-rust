/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#include <dlfcn.h>
#include <os/lock.h>
#include <stdint.h>
#include <errno.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <pthread.h>
#include <unistd.h>
static _Atomic int notifier_alive;
static void *(*notifier_entry)(void *);
static void *joined_entry(void *argument) {
  atomic_store(&notifier_alive, 1);
  void *value = notifier_entry(argument);
  /* The notifier has stopped work but its native return has not completed. */
  usleep(20000);
  atomic_store(&notifier_alive, 0);
  return value;
}
static int create_notifier(pthread_t *thread, const pthread_attr_t *attributes,
                           void *(*entry)(void *), void *argument) {
  notifier_entry = entry;
  return pthread_create(thread, attributes, joined_entry, argument);
}
#define pthread_create create_notifier
#include "instance_timer.h"
#undef pthread_create
static _Atomic unsigned count;
static void tick(int signal_number) { (void)signal_number; atomic_fetch_add(&count, 1); }
#define API __attribute__((visibility("default")))
API int timer_retiring(void) { return atomic_load(&notifier_alive); }
API unsigned timer_count(void) { return atomic_load(&count); }
API int timer_start(long first, long period) {
  gerbil_timer_init_owner();
  struct sigaction action = {0};
  action.sa_handler = tick;
  sigemptyset(&action.sa_mask);
  if (sigaction(SIGALRM, &action, NULL) != 0) return -1;
  struct itimerval interval = {{0, period}, {0, first}};
  return setitimer(ITIMER_REAL, &interval, NULL);
}
API int timer_mask(int blocked) {
  sigset_t set;
  sigemptyset(&set);
  sigaddset(&set, SIGALRM);
  return pthread_sigmask(blocked ? SIG_BLOCK : SIG_UNBLOCK, &set, NULL);
}
API int timer_invalid(void) {
  struct sigaction action = {0};
  action.sa_handler = tick;
  action.sa_flags = SA_SIGINFO;
  sigemptyset(&action.sa_mask);
  if (sigaction(SIGALRM, &action, NULL) != -1 || errno != ENOTSUP) return 0;
  struct itimerval interval = {{0, 1000}, {0, -1}};
  return setitimer(ITIMER_REAL, &interval, NULL) == -1 && errno == EINVAL;
}
API void timer_close(void) { if (gerbil_timer_cleanup() != 0 || gerbil_timer_status() != 0) abort(); }
