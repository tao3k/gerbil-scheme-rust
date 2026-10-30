/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#include <assert.h>
#include <dlfcn.h>
#include <signal.h>
#include <stdio.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>
typedef struct {
  void *image;
  unsigned (*count)(void);
  int (*start)(long,long);
  int (*mask)(int);
  int (*invalid)(void);
  void (*close)(void);
  int (*retiring)(void);
} Timer;
static volatile sig_atomic_t host_ticks;
static void host_tick(int number) { (void)number; ++host_ticks; }
static double now(void) {
  struct timespec value;
  assert(clock_gettime(CLOCK_MONOTONIC, &value) == 0);
  return (double)value.tv_sec + value.tv_nsec * 1e-9;
}
static Timer load(const char *path) {
  void *image = dlopen(path, RTLD_NOW | RTLD_LOCAL);
  if (!image) fprintf(stderr, "%s\n", dlerror());
  assert(image);
  Timer timer = {image, dlsym(image,"timer_count"), dlsym(image,"timer_start"),
    dlsym(image,"timer_mask"), dlsym(image,"timer_invalid"), dlsym(image,"timer_close"), dlsym(image,"timer_retiring")};
  assert(timer.count && timer.start && timer.mask && timer.invalid && timer.close);
  return timer;
}
static void wait_ticks(Timer timer, unsigned minimum) {
  double deadline = now() + 2;
  while (timer.count() < minimum) {
    assert(now() < deadline);
    usleep(500);
  }
}
int main(int argc, char **argv) {
  assert(argc == 3);
  struct sigaction original, action = {0}, current;
  struct itimerval original_timer, host_timer = {{0,40000},{0,40000}}, current_timer;
  assert(sigaction(SIGALRM,NULL,&original) == 0);
  action.sa_handler = host_tick;
  sigemptyset(&action.sa_mask);
  assert(sigaction(SIGALRM,&action,NULL) == 0);
  assert(setitimer(ITIMER_REAL,&host_timer,&original_timer) == 0);
  Timer a = load(argv[1]), b = load(argv[2]);
  assert(a.start(2000,2000) == 0 && b.start(3000,3000) == 0);
  wait_ticks(a,5); wait_ticks(b,5);
  assert(sigaction(SIGALRM,NULL,&current) == 0 && current.sa_handler == host_tick);
  assert(getitimer(ITIMER_REAL,&current_timer) == 0 && current_timer.it_interval.tv_usec == 40000);
  assert(a.start(0,0) == 0);
  unsigned once = a.count();
  assert(a.start(2000,0) == 0);
  wait_ticks(a,once+1);
  unsigned one_shot = a.count(), continuing = b.count();
  wait_ticks(b,continuing+4);
  assert(a.count() == one_shot);
  assert(a.start(2000,2000) == 0);
  assert(a.mask(1) == 0);
  unsigned stopped = a.count(), other = b.count();
  wait_ticks(b,other+5);
  assert(a.count() == stopped);
  assert(a.mask(0) == 0);
  wait_ticks(a,stopped+2);
  assert(a.invalid());
  other = b.count();
  a.close(); assert(!a.retiring()); stopped = a.count();
  wait_ticks(b,other+5);
  assert(a.count() == stopped);
  a.close();
  assert(dlclose(a.image) == 0);
  wait_ticks(b,other+7);
  b.close(); assert(!b.retiring()); assert(dlclose(b.image) == 0);
  double deadline = now()+2;
  while (host_ticks == 0) { assert(now()<deadline); usleep(500); }
  assert(sigaction(SIGALRM,NULL,&current) == 0 && current.sa_handler == host_tick);
  assert(getitimer(ITIMER_REAL,&current_timer) == 0 && current_timer.it_interval.tv_usec == 40000);
  assert(setitimer(ITIMER_REAL,&original_timer,NULL) == 0);
  assert(sigaction(SIGALRM,&original,NULL) == 0);
  puts("INSTANCE-TIMER-ISOLATION-OK host-mask-retirement-unload");
  return 0;
}
