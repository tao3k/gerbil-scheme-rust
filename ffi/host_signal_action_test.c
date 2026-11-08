/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#define _GNU_SOURCE
#include <assert.h>
#include <signal.h>
#include <string.h>

/* The paired Gambit ABI's external helper, in a separate translation unit. */
void ___set_signal_handler(int signal_number, void (*handler)(int)) {
  struct sigaction action;
  memset(&action, 0, sizeof(action));
  sigemptyset(&action.sa_mask);
  action.sa_handler = handler;
  assert(sigaction(signal_number, &action, NULL) == 0);
}

/* Permit the one-variable, unwrapped link control to use the same objects. */
void __real____set_signal_handler(int signal_number, void (*handler)(int)) {
  ___set_signal_handler(signal_number, handler);
}
