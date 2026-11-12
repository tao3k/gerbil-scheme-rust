/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#define _GNU_SOURCE
#define GERBIL_SCHEME_RUST_SCOPED_SIGNALS
#include <assert.h>
#include <string.h>
#include <sys/wait.h>
#include "host_io.h"

extern void gerbil_test_child_signal(int, void (*)(int));
static void host_handler(int signal_number) { (void)signal_number; }
static void updated_host_handler(int signal_number) { (void)signal_number; }
static void foreign_reaper(int signal_number) {
  (void)signal_number;
  int saved = errno;
  while (waitpid(-1, NULL, WNOHANG) > 0) {}
  errno = saved;
}

int main(void) {
  alarm(5);
  gerbil_host_io io;
  assert(gerbil_host_io_capture(&io) == 0);
  assert(gerbil_host_io_restore(&io) == 0);
  assert(gerbil_host_affinity_restore(&io.affinity) == 0);
  gerbil_host_sigpipe pipe_scope;
  assert(gerbil_host_sigpipe_begin(&pipe_scope) == 0);
  assert(gerbil_host_sigpipe_end(&pipe_scope) == 0);
  struct sigaction original, action, current;
  memset(&action, 0, sizeof(action));
  sigemptyset(&action.sa_mask);
  action.sa_handler = host_handler;
  assert(sigaction(SIGCHLD, &action, &original) == 0);
  sigset_t before, after;
  assert(sigprocmask(SIG_SETMASK, NULL, &before) == 0);
  int gate[2];
  assert(pipe(gate) == 0);
  pid_t child = fork();
  assert(child >= 0);
  if (child == 0) {
    close(gate[1]);
    char ready;
    if (read(gate[0], &ready, 1) != 1) _exit(80);
    _exit(37);
  }
  close(gate[0]);
  gerbil_host_signals saved;
  assert(gerbil_host_signals_capture(&saved) == 0);
  gerbil_host_child_scope_begin();
  gerbil_test_child_signal(SIGCHLD, foreign_reaper);
  assert(sigaction(SIGCHLD, NULL, &current) == 0);
  if (current.sa_handler != host_handler) {
    /* The unbound archive is the one-variable negative control. */
    close(gate[1]);
    while (waitpid(child, NULL, 0) < 0 && errno == EINTR) {}
    puts("HOST-CHILD-POLICY-CHANGED");
    return 42;
  }
  assert(gerbil_host_child_handler_seen && gerbil_host_child_handler == foreign_reaper);
  /* A genuine host update during setup must survive restoration. */
  action.sa_handler = updated_host_handler;
  assert(sigaction(SIGCHLD, &action, NULL) == 0);
  assert(write(gate[1], "x", 1) == 1);
  close(gate[1]);
  int status;
  pid_t waited;
  do { waited = waitpid(child, &status, 0); } while (waited < 0 && errno == EINTR);
  assert(waited == child && WIFEXITED(status) && WEXITSTATUS(status) == 37);
  gerbil_test_child_signal(SIGCHLD, SIG_DFL); /* native cleanup attempt */
  assert(gerbil_host_signals_restore(&saved) == 0);
  gerbil_host_child_scope_end();
  assert(sigaction(SIGCHLD, NULL, &current) == 0 &&
         current.sa_handler == updated_host_handler);
  assert(sigprocmask(SIG_SETMASK, NULL, &after) == 0);
  for (int n = 1; n < NSIG; ++n)
    assert(sigismember(&before, n) == sigismember(&after, n));
  /* Explicit SDK operations outside the scope still use the real helper. */
  gerbil_test_child_signal(SIGCHLD, SIG_DFL);
  assert(sigaction(SIGCHLD, NULL, &current) == 0 && current.sa_handler == SIG_DFL);
  assert(sigaction(SIGCHLD, &original, NULL) == 0);
  puts("HOST-CHILD-SCOPE-OK wait-status host-update unchanged-mask");
  return 0;
}
