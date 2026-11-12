/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
/* Gambit startup must not mutate inherited host open-file descriptions.
 * Merely restoring F_SETFL afterwards races other processes sharing them. */
#if defined(__unix__) || defined(__APPLE__)
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <unistd.h>
#if defined(GERBIL_SCHEME_RUST_SCOPED_SIGNALS)
#include <stdatomic.h>
/* Bound only inside the paired Gambit archive. These scopes never replace
 * host libc calls or mask signals on unrelated Rust threads. */
#if defined(__linux__)
static _Thread_local unsigned gerbil_host_signal_scope_depth;
#endif
/* The unique VM owner serializes scope entry and exit. This state belongs to
 * its image, not caller TLS: Darwin retains a TLS key for C thread-local data
 * while image code remains leased after owner join. SDK helper reads may come
 * from other threads, so the image-owned state uses atomic access. */
typedef void (*gerbil_host_signal_handler)(int);
static _Atomic unsigned gerbil_host_child_scope_depth;
static _Atomic(gerbil_host_signal_handler) gerbil_host_child_handler;
static _Atomic int gerbil_host_child_handler_seen;
extern void ___set_signal_handler(int, void (*)(int));
void __wrap____set_signal_handler(int signal_number, void (*handler)(int)) {
  if (signal_number == SIGCHLD && gerbil_host_child_scope_depth != 0) {
    gerbil_host_child_handler = handler;
    gerbil_host_child_handler_seen = 1;
    return;
  }
#if defined(__linux__)
  if (signal_number == SIGPIPE && gerbil_host_signal_scope_depth != 0) return;
#endif
  ___set_signal_handler(signal_number, handler);
}
static inline void gerbil_host_child_scope_begin(void) {
  if (gerbil_host_child_scope_depth == 0) gerbil_host_child_handler_seen = 0;
  gerbil_host_child_scope_depth++;
}
static inline void gerbil_host_child_scope_end(void) { gerbil_host_child_scope_depth--; }
#else
static inline void gerbil_host_child_scope_begin(void) {}
static inline void gerbil_host_child_scope_end(void) {}
#endif
/* Gambit binds its owner thread to one CPU. Native compiler children must
 * inherit the embedding host's original allowed CPUs, including restrictions
 * set by its container or caller. Never reset to an invented all-CPU mask. */
#if defined(__linux__)
#include <sched.h>
#include <pthread.h>
#include <time.h>
/* Gambit resets SIGPIPE before retiring its I/O devices. Defer signals from
 * that owner-local cleanup until the caller's disposition has been restored. */
typedef struct { sigset_t mask; int pending; } gerbil_host_sigpipe;
static int gerbil_host_sigpipe_begin(gerbil_host_sigpipe *scope) {
  sigset_t pipe, pending;
  sigemptyset(&pipe);
  sigaddset(&pipe, SIGPIPE);
  int status = pthread_sigmask(SIG_BLOCK, &pipe, &scope->mask);
  if (status != 0) { errno = status; return -1; }
  if (sigpending(&pending) != 0) {
    int saved = errno;
    pthread_sigmask(SIG_SETMASK, &scope->mask, NULL);
    errno = saved;
    return -1;
  }
  scope->pending = sigismember(&pending, SIGPIPE);
#if defined(GERBIL_SCHEME_RUST_SCOPED_SIGNALS)
  gerbil_host_signal_scope_depth++;
#endif
  return 0;
}
static int gerbil_host_sigpipe_end(const gerbil_host_sigpipe *scope) {
  int result = 0;
  if (!scope->pending) {
    sigset_t pipe;
    sigemptyset(&pipe);
    sigaddset(&pipe, SIGPIPE);
    const struct timespec now = {0, 0};
    int status;
    do { status = sigtimedwait(&pipe, NULL, &now); } while (status < 0 && errno == EINTR);
    if (status < 0 && errno != EAGAIN) result = -1;
  }
#if defined(GERBIL_SCHEME_RUST_SCOPED_SIGNALS)
  gerbil_host_signal_scope_depth--;
#endif
  int status = pthread_sigmask(SIG_SETMASK, &scope->mask, NULL);
  if (status != 0) { errno = status; result = -1; }
  return result;
}
typedef cpu_set_t gerbil_host_affinity;
static int gerbil_host_affinity_capture(gerbil_host_affinity *affinity) {
  return sched_getaffinity(0, sizeof(*affinity), affinity);
}
static int gerbil_host_affinity_restore(const gerbil_host_affinity *affinity) {
  return sched_setaffinity(0, sizeof(*affinity), affinity);
}
#else
typedef struct { int unused; } gerbil_host_sigpipe;
static int gerbil_host_sigpipe_begin(gerbil_host_sigpipe *scope) { (void)scope; return 0; }
static int gerbil_host_sigpipe_end(const gerbil_host_sigpipe *scope) { (void)scope; return 0; }
typedef struct { int unused; } gerbil_host_affinity;
static int gerbil_host_affinity_capture(gerbil_host_affinity *affinity) { (void)affinity; return 0; }
static int gerbil_host_affinity_restore(const gerbil_host_affinity *affinity) { (void)affinity; return 0; }
#endif
/* Process control belongs to the embedding host. Leave the SDK's internal
 * timer signals alone so Scheme green-thread scheduling remains native. */
typedef struct {
  struct sigaction actions[4];
  unsigned saved;
} gerbil_host_signals;
static const int gerbil_host_signal_numbers[4] = { SIGCHLD, SIGINT, SIGTERM, SIGPIPE };
static int gerbil_host_signals_capture(gerbil_host_signals *signals) {
  memset(signals, 0, sizeof(*signals));
  for (unsigned i = 0; i < 4; ++i) {
    if (sigaction(gerbil_host_signal_numbers[i], NULL, &signals->actions[i]) < 0)
      return -1;
    signals->saved++;
  }
  return 0;
}
static int gerbil_host_signals_restore(const gerbil_host_signals *signals) {
  int result = 0;
  for (unsigned i = 0; i < signals->saved; ++i) {
#if defined(GERBIL_SCHEME_RUST_SCOPED_SIGNALS)
    /* Setup/cleanup did not change SIGCHLD. Do not overwrite a concurrent
     * host update with the snapshot taken before the operation. */
    if (gerbil_host_signal_numbers[i] == SIGCHLD && gerbil_host_child_scope_depth != 0)
      continue;
#endif
#if defined(__linux__) && defined(GERBIL_SCHEME_RUST_SCOPED_SIGNALS)
    /* Gambit did not change this action in the cleanup scope. Preserve any
     * legitimate concurrent host update instead of restoring a stale action. */
    if (gerbil_host_signal_numbers[i] == SIGPIPE && gerbil_host_signal_scope_depth != 0)
      continue;
#endif
    if (sigaction(gerbil_host_signal_numbers[i], &signals->actions[i], NULL) < 0)
      result = -1;
  }
  return result;
}
typedef struct {
  int saved[3];
  int descriptor_flags[3];
  FILE *private_stream[3];
  off_t forwarded[3];
  int forward_error;
  gerbil_host_signals signals;
  gerbil_host_affinity affinity;
} gerbil_host_io;

/* Read without moving the open-file-description offset shared by Scheme and
 * compiler children. Each byte is forwarded once, including the final drain. */
static int gerbil_host_io_forward(gerbil_host_io *host) {
  if (host->forward_error) return -1;
  for (int fd = 1; fd < 3; ++fd) {
    if (host->private_stream[fd] == NULL || host->saved[fd] < 0) continue;
    char bytes[8192];
    for (;;) {
      ssize_t count = pread(fileno(host->private_stream[fd]), bytes, sizeof(bytes), host->forwarded[fd]);
      if (count < 0 && errno == EINTR) continue;
      if (count < 0) goto failed;
      if (count == 0) break;
      ssize_t offset = 0;
      while (offset < count) {
        ssize_t written = write(host->saved[fd], bytes + offset, (size_t)(count - offset));
        if (written < 0 && errno == EINTR) continue;
        if (written <= 0) goto failed;
        offset += written;
        host->forwarded[fd] += written;
      }
    }
  }
  return 0;
failed:
  host->forward_error = 1;
  return -1;
}

static FILE *gerbil_private_stream(void) {
  FILE *stream = tmpfile();
  if (stream == NULL) return NULL;
  int fd = fileno(stream);
  if (fd < 3) {
    int moved = fcntl(fd, F_DUPFD, 3);
    if (moved < 0) { fclose(stream); return NULL; }
    FILE *high = fdopen(moved, "w+b");
    if (high == NULL) { close(moved); fclose(stream); return NULL; }
    fclose(stream);
    stream = high;
    fd = moved;
  }
  if (fcntl(fd, F_SETFD, FD_CLOEXEC) < 0) { fclose(stream); return NULL; }
  return stream;
}

static int gerbil_host_io_restore(gerbil_host_io *host) {
  int result = gerbil_host_io_forward(host);
  for (int fd = 0; fd < 3; ++fd) {
    if (host->saved[fd] >= 0) {
      if (dup2(host->saved[fd], fd) < 0 ||
          fcntl(fd, F_SETFD, host->descriptor_flags[fd]) < 0) result = -1;
      close(host->saved[fd]);
      host->saved[fd] = -1;
    } else if (host->descriptor_flags[fd] == -1) {
      close(fd); /* It was closed before native setup, and remains host-closed. */
    }
  }
  if (gerbil_host_signals_restore(&host->signals) != 0) result = -1;
  if (gerbil_host_affinity_restore(&host->affinity) != 0) result = -1;
  for (int fd = 0; fd < 3; ++fd) {
    FILE *stream = host->private_stream[fd];
    if (stream == NULL) continue;
    if (fclose(stream) != 0) result = -1;
    host->private_stream[fd] = NULL;
  }
  return result;
}

static int gerbil_host_io_capture(gerbil_host_io *host) {
  memset(host, 0, sizeof(*host));
  for (int fd = 0; fd < 3; ++fd) {
    host->saved[fd] = -1;
    host->descriptor_flags[fd] = -2; /* not yet inspected */
  }
  if (gerbil_host_affinity_capture(&host->affinity) != 0 ||
      gerbil_host_signals_capture(&host->signals) != 0) return -1;
  for (int fd = 0; fd < 3; ++fd) {
    int flags = fcntl(fd, F_GETFD);
    if (flags < 0 && errno == EBADF) { host->descriptor_flags[fd] = -1; continue; }
    if (flags < 0) goto failed;
    host->descriptor_flags[fd] = flags;
  }
  for (int fd = 0; fd < 3; ++fd) {
    int flags = host->descriptor_flags[fd];
    if (flags < 0) continue;
    host->saved[fd] = fcntl(fd, F_DUPFD, 3);
    if (host->saved[fd] < 0 || fcntl(host->saved[fd], F_SETFD, FD_CLOEXEC) < 0) goto failed;
    host->private_stream[fd] = gerbil_private_stream();
    if (host->private_stream[fd] == NULL ||
        dup2(fileno(host->private_stream[fd]), fd) < 0 ||
        fcntl(fd, F_SETFD, flags) < 0) goto failed;
  }
  return 0;
failed:
  gerbil_host_io_restore(host);
  return -1;
}
#else
static inline void gerbil_host_child_scope_begin(void) {}
static inline void gerbil_host_child_scope_end(void) {}
typedef struct { int unused; } gerbil_host_sigpipe;
static int gerbil_host_sigpipe_begin(gerbil_host_sigpipe *scope) { (void)scope; return 0; }
static int gerbil_host_sigpipe_end(const gerbil_host_sigpipe *scope) { (void)scope; return 0; }
typedef struct { int unused; } gerbil_host_affinity;
static int gerbil_host_affinity_capture(gerbil_host_affinity *affinity) { (void)affinity; return 0; }
static int gerbil_host_affinity_restore(const gerbil_host_affinity *affinity) { (void)affinity; return 0; }
typedef struct { int unused; } gerbil_host_signals;
static int gerbil_host_signals_capture(gerbil_host_signals *signals) { (void)signals; return 0; }
static int gerbil_host_signals_restore(const gerbil_host_signals *signals) { (void)signals; return 0; }
typedef struct { gerbil_host_signals signals; } gerbil_host_io;
static int gerbil_host_io_capture(gerbil_host_io *host) { (void)host; return 0; }
static int gerbil_host_io_restore(gerbil_host_io *host) { (void)host; return 0; }
#endif
