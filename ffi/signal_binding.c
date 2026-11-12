/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
/* This shim is localized together with each Gambit caller on Mach-O.
 * Its private alias binds SDK calls without interposing the host libc. */
extern void __wrap____set_signal_handler(int, void (*)(int));
void gerbil_scoped_signal_binding(int signal_number, void (*handler)(int)) {
  __wrap____set_signal_handler(signal_number, handler);
}
