/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
/* Gambit startup must not mutate inherited host open-file descriptions.
 * Merely restoring F_SETFL afterwards races other processes sharing them. */
#if defined(__unix__) || defined(__APPLE__)
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <unistd.h>
/* Process control belongs to the embedding host. Leave the SDK's internal
 * timer signals alone so Scheme green-thread scheduling remains native. */
typedef struct {
  struct sigaction actions[3];
  unsigned saved;
} gerbil_host_signals;
static const int gerbil_host_signal_numbers[3] = { SIGCHLD, SIGINT, SIGTERM };
static int gerbil_host_signals_capture(gerbil_host_signals *signals) {
  memset(signals, 0, sizeof(*signals));
  for (unsigned i = 0; i < 3; ++i) {
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
  gerbil_host_signals signals;
} gerbil_host_io;

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
  int result = 0;
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
  for (int fd = 0; fd < 3; ++fd) {
    FILE *stream = host->private_stream[fd];
    if (stream == NULL) continue;
    if (fd != 0 && result == 0) {
      char bytes[8192];
      if (fseek(stream, 0, SEEK_SET) != 0) result = -1;
      size_t length;
      while (result == 0 && (length = fread(bytes, 1, sizeof(bytes), stream)) > 0) {
        size_t offset = 0;
        while (offset < length) {
          ssize_t written = write(fd, bytes + offset, length - offset);
          if (written < 0 && errno == EINTR) continue;
          if (written <= 0) { result = -1; break; }
          offset += (size_t)written;
        }
      }
      if (ferror(stream)) result = -1;
    }
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
  if (gerbil_host_signals_capture(&host->signals) != 0) return -1;
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
typedef struct { int unused; } gerbil_host_signals;
static int gerbil_host_signals_capture(gerbil_host_signals *signals) { (void)signals; return 0; }
static int gerbil_host_signals_restore(const gerbil_host_signals *signals) { (void)signals; return 0; }
typedef struct { gerbil_host_signals signals; } gerbil_host_io;
static int gerbil_host_io_capture(gerbil_host_io *host) { (void)host; return 0; }
static int gerbil_host_io_restore(gerbil_host_io *host) { (void)host; return 0; }
#endif
