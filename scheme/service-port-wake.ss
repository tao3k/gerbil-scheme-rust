;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Included after service records: one Scheme-owned input event for host and tasks.
(extern service-notify-fd)
(begin-foreign
 (namespace ("gerbil-scheme-rust/scheme/async-service#" service-notify-fd))
 (c-declare #<<c-end
#include <unistd.h>
#include <errno.h>
static int gerbil_service_notify_fd(int fd) {
  unsigned char byte = 1;
  ssize_t result;
  do { result = write(fd, &byte, 1); } while (result < 0 && errno == EINTR);
  return result == 1 || (result < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) ? 0 : -1;
}
c-end
 )
 (define service-notify-fd (c-lambda (int) int "gerbil_service_notify_fd")))
(def (service-port-notify! service)
  (unless (scheme-service-notified service)
    ;; The Rust-owned descriptor is nonblocking and suppresses SIGPIPE. A full
    ;; pipe is already a level-triggered wake, not permission to drop work.
    (unless (zero? (service-notify-fd (scheme-service-write-fd service)))
      (set! (scheme-service-broken service) #t)
      (error "Scheme native wake write failed"))
    (set! (scheme-service-notified service) #t)))
(def (service-port-read! service)
  (when (eof-object? (read-u8 (scheme-service-port service)))
    (set! (scheme-service-broken service) #t)
    (error "Scheme native wake port closed")))
(def (service-port-drain! service)
  (let loop ((left 4096))
    (when (and (positive? left) (u8-ready? (scheme-service-port service)))
      (service-port-read! service) (loop (1- left))))
  (set! (scheme-service-notified service) #f))
(def (service-port-wait! service microseconds)
  (when (scheme-service-broken service) (error "Scheme native wake port failed"))
  (if (or (not (service-fifo-empty? (scheme-service-ready service)))
          (not (service-fifo-empty? (scheme-service-short-ready service)))) 0
    (let ((timeout (cons 'native-wake-timeout [])) (port (scheme-service-port service)))
      (with-catch (lambda (failure) (if (eq? failure timeout) 0 (raise failure)))
        (lambda ()
          (dynamic-wind
            (lambda ()
              (input-port-timeout-set! port
                (and (positive? microseconds) (/ microseconds 1000000.0))
                (lambda () (raise timeout))))
            (lambda () (service-port-read! service) (service-port-drain! service) 0)
            (lambda () (input-port-timeout-set! port #f))))))))
(def (service-port-mode! service mode)
  (unless (and (memv mode '(0 1)) (not (scheme-service-closing service))
               (zero? (table-length (scheme-service-jobs service)))
               (gerbil-rs-budget-idle? (scheme-service-budget service)))
    (error "Scheme wake mode requires idle service"))
  (unless (eq? (= mode 1) (scheme-service-direct service))
    (if (= mode 1)
      (begin
        (set! (scheme-service-direct service) #t)
        (service-port-notify! service)
        (thread-join! (scheme-service-watcher service))
        (set! (scheme-service-watcher service) #f)
        (service-port-drain! service))
      (begin
        (service-port-drain! service)
        (set! (scheme-service-direct service) #f)
        (set! (scheme-service-watcher service) (spawn (lambda () (service-watch service)))))))
  mode)
