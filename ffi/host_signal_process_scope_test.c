/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#define _GNU_SOURCE
#define GERBIL_SCHEME_RUST_SCOPED_SIGNALS
#include <assert.h>
#include <stdint.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/uio.h>
#include "host_io.h"

extern void ___set_signal_handler(int, void (*)(int));
static pthread_barrier_t resetting;
static int writer;
static volatile sig_atomic_t delivered;

static void host_handler(int signal_number) {
  (void)signal_number;
  delivered++;
}

static void *concurrent_write(void *unused) {
  (void)unused;
  sigset_t mask;
  assert(pthread_sigmask(SIG_SETMASK, NULL, &mask) == 0);
  assert(sigismember(&mask, SIGPIPE) == 0);
  int status = pthread_barrier_wait(&resetting);
  assert(status == 0 || status == PTHREAD_BARRIER_SERIAL_THREAD);
  struct iovec bytes[2] = {{.iov_base = (void *)"closed", .iov_len = 6},
                           {.iov_base = (void *)"peer", .iov_len = 4}};
  assert(writev(writer, bytes, 2) == -1 && errno == EPIPE);
  return NULL;
}

int main(void) {
  alarm(5);
  gerbil_host_io io;
  assert(gerbil_host_io_capture(&io) == 0);
  assert(gerbil_host_io_restore(&io) == 0);
  struct sigaction original_action, action;
  memset(&action, 0, sizeof(action));
  sigemptyset(&action.sa_mask);
  action.sa_handler = SIG_IGN;
  assert(sigaction(SIGPIPE, &action, &original_action) == 0);
  sigset_t original_mask, caller_mask, after;
  assert(pthread_sigmask(SIG_SETMASK, NULL, &original_mask) == 0);
  caller_mask = original_mask;
  sigdelset(&caller_mask, SIGPIPE);
  assert(pthread_sigmask(SIG_SETMASK, &caller_mask, NULL) == 0);
  int pair[2];
  assert(socketpair(AF_UNIX, SOCK_STREAM, 0, pair) == 0);
  assert(close(pair[0]) == 0);
  writer = pair[1];
  assert(pthread_barrier_init(&resetting, NULL, 2) == 0);
  pthread_t worker;
  /* The host worker inherits the unblocked caller mask, not the owner's guard. */
  assert(pthread_create(&worker, NULL, concurrent_write, NULL) == 0);
  gerbil_host_signals signals;
  assert(gerbil_host_signals_capture(&signals) == 0);
  gerbil_host_sigpipe scope;
  assert(gerbil_host_sigpipe_begin(&scope) == 0);
  ___set_signal_handler(SIGPIPE, SIG_DFL);
  int status = pthread_barrier_wait(&resetting);
  assert(status == 0 || status == PTHREAD_BARRIER_SERIAL_THREAD);
  assert(pthread_join(worker, NULL) == 0);
  assert(sigaction(SIGPIPE, NULL, &action) == 0 && action.sa_handler == SIG_IGN);
  /* A real host update remains effective even on the guarded owner thread. */
  action.sa_handler = host_handler;
  assert(sigaction(SIGPIPE, &action, NULL) == 0);
  assert(write(writer, "owner", 5) == -1 && errno == EPIPE);
  assert(gerbil_host_signals_restore(&signals) == 0);
  assert(gerbil_host_sigpipe_end(&scope) == 0);
  assert(delivered == 0);
  assert(sigaction(SIGPIPE, NULL, &action) == 0 && action.sa_handler == host_handler);
  assert(pthread_sigmask(SIG_SETMASK, NULL, &after) == 0);
  for (int signal_number = 1; signal_number < NSIG; ++signal_number)
    assert(sigismember(&after, signal_number) == sigismember(&caller_mask, signal_number));
  /* Ordinary Gambit signal operations outside cleanup still reach the OS. */
  ___set_signal_handler(SIGPIPE, SIG_DFL);
  assert(sigaction(SIGPIPE, NULL, &action) == 0 && action.sa_handler == SIG_DFL);
  assert(close(writer) == 0);
  assert(pthread_barrier_destroy(&resetting) == 0);
  assert(sigaction(SIGPIPE, &original_action, NULL) == 0);
  assert(pthread_sigmask(SIG_SETMASK, &original_mask, NULL) == 0);
  puts("HOST-SIGNAL-PROCESS-SCOPE-OK concurrent-writev host-policy unchanged-mask");
  return 0;
}
