/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#include <assert.h>
#include <signal.h>
extern void ___set_signal_handler(int, void (*)(int));
extern int gerbil_test_program_binding;

void gerbil_test_native_signal_cleanup(void) {
  assert(gerbil_test_program_binding == 41);
  ___set_signal_handler(SIGPIPE, SIG_DFL);
}
