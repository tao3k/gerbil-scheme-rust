/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#define _GNU_SOURCE
#include <assert.h>
#include <stdint.h>
#include <string.h>
#include "host_io.h"

static void cleanup_write(int existing_pending) {
  sigset_t original_mask, caller_mask, after, pipe;
  struct sigaction original_action, action;
  assert(pthread_sigmask(SIG_SETMASK, NULL, &original_mask) == 0);
  caller_mask = original_mask;
  sigaddset(&caller_mask, SIGCHLD);
  if (existing_pending) sigaddset(&caller_mask, SIGPIPE);
  else sigdelset(&caller_mask, SIGPIPE);
  assert(pthread_sigmask(SIG_SETMASK, &caller_mask, NULL) == 0);
  memset(&action, 0, sizeof(action));
  sigemptyset(&action.sa_mask);
  action.sa_handler = existing_pending ? SIG_DFL : SIG_IGN;
  assert(sigaction(SIGPIPE, &action, &original_action) == 0);
  if (existing_pending) assert(raise(SIGPIPE) == 0);

  gerbil_host_signals host;
  assert(gerbil_host_signals_capture(&host) == 0);
#ifndef GERBIL_TEST_UNGUARDED_SIGPIPE
  gerbil_host_sigpipe scope;
  assert(gerbil_host_sigpipe_begin(&scope) == 0);
#endif
  /* Gambit's io_module_cleanup resets handlers before retiring devices. */
  action.sa_handler = SIG_DFL;
  assert(sigaction(SIGPIPE, &action, NULL) == 0);
  sigemptyset(&pipe);
  sigaddset(&pipe, SIGCHLD);
  assert(pthread_sigmask(SIG_UNBLOCK, &pipe, NULL) == 0);
  int descriptors[2];
  assert(pipe2(descriptors, O_CLOEXEC) == 0);
  assert(close(descriptors[0]) == 0);
  assert(write(descriptors[1], "closed", 6) == -1 && errno == EPIPE);
  assert(close(descriptors[1]) == 0);
  assert(gerbil_host_signals_restore(&host) == 0);
#ifndef GERBIL_TEST_UNGUARDED_SIGPIPE
  assert(gerbil_host_sigpipe_end(&scope) == 0);
#endif
  assert(pthread_sigmask(SIG_SETMASK, NULL, &after) == 0);
  for (int signal_number = 1; signal_number < NSIG; ++signal_number)
    assert(sigismember(&caller_mask, signal_number) == sigismember(&after, signal_number));
  assert(sigpending(&after) == 0);
  assert(sigismember(&after, SIGPIPE) == existing_pending);
  if (existing_pending) {
    sigemptyset(&pipe);
    sigaddset(&pipe, SIGPIPE);
    const struct timespec now = {0, 0};
    assert(sigtimedwait(&pipe, NULL, &now) == SIGPIPE);
  }
  assert(sigaction(SIGPIPE, &original_action, NULL) == 0);
  assert(pthread_sigmask(SIG_SETMASK, &original_mask, NULL) == 0);
}

int main(void) {
  gerbil_host_io io;
  assert(gerbil_host_io_capture(&io) == 0);
  assert(gerbil_host_io_restore(&io) == 0);
  gerbil_host_sigpipe scope;
  assert(gerbil_host_sigpipe_begin(&scope) == 0);
  assert(gerbil_host_sigpipe_end(&scope) == 0);
  cleanup_write(0);
  cleanup_write(1);
  puts("HOST-SIGPIPE-SCOPE-OK original-mask original-pending closed-pipe");
  return 0;
}
