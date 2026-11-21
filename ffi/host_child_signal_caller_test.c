/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
extern void ___set_signal_handler(int, void (*)(int));
void gerbil_test_child_signal(int signal_number, void (*handler)(int)) {
  ___set_signal_handler(signal_number, handler);
}
