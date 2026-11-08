/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
/* Gambit startup must not mutate inherited host open-file descriptions.
 * Merely restoring F_SETFL afterwards races other processes sharing them. */
#if defined(__unix__) || defined(__APPLE__)
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <unistd.h>
/* Gambit binds its owner thread to one CPU. Native compiler children must
 * inherit the embedding host's original allowed CPUs, including restrictions
 * set by its container or caller. Never reset to an invented all-CPU mask. */
#if defined(__linux__)
#include <sched.h>
typedef cpu_set_t gerbil_host_affinity;
static int gerbil_host_affinity_capture(gerbil_host_affinity *affinity) {
  return sched_getaffinity(0, sizeof(*affinity), affinity);
}
static int gerbil_host_affinity_restore(const gerbil_host_affinity *affinity) {
  return sched_setaffinity(0, sizeof(*affinity), affinity);
}
#else
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
