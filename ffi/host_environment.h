/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
/* Gambit's USE_environ cleanup can free its extended pointer table without
 * replacing the process's environ. The host must own a surviving copy before
 * native teardown. Values, including host changes between calls, are retained. */
#ifndef GERBIL_SCHEME_RUST_HOST_ENVIRONMENT_H
#define GERBIL_SCHEME_RUST_HOST_ENVIRONMENT_H
#if defined(__unix__) || defined(__APPLE__)
#include <stdlib.h>
#include <string.h>
#ifdef __APPLE__
#include <crt_externs.h>
static char ***gerbil_host_environment_location(void) { return _NSGetEnviron(); }
#else
extern char **environ;
static char ***gerbil_host_environment_location(void) { return &environ; }
#endif

typedef struct { char **entries; } gerbil_host_environment;
static void gerbil_host_environment_discard(gerbil_host_environment *snapshot) {
  if (snapshot->entries == NULL) return;
  for (size_t i = 0; snapshot->entries[i] != NULL; ++i) free(snapshot->entries[i]);
  free(snapshot->entries);
  snapshot->entries = NULL;
}
static int gerbil_host_environment_capture(gerbil_host_environment *snapshot) {
  char **current = *gerbil_host_environment_location();
  size_t count = 0;
  snapshot->entries = NULL;
  while (current != NULL && current[count] != NULL) {
    if (count == SIZE_MAX / sizeof(char *) - 1) return -1;
    ++count;
  }
  snapshot->entries = calloc(count + 1, sizeof(char *));
  if (snapshot->entries == NULL) return -1;
  for (size_t i = 0; i < count; ++i) {
    snapshot->entries[i] = strdup(current[i]);
    if (snapshot->entries[i] == NULL) {
      gerbil_host_environment_discard(snapshot);
      return -1;
    }
  }
  return 0;
}
static void gerbil_host_environment_restore(gerbil_host_environment *snapshot) {
  /* Transfer to the process environment's lifetime after Gambit has released
   * its own table. Installing this earlier would let Gambit free the host copy. */
  *gerbil_host_environment_location() = snapshot->entries;
  snapshot->entries = NULL;
}
#else
typedef struct { int unused; } gerbil_host_environment;
static void gerbil_host_environment_discard(gerbil_host_environment *snapshot) { (void)snapshot; }
static int gerbil_host_environment_capture(gerbil_host_environment *snapshot) { (void)snapshot; return 0; }
static void gerbil_host_environment_restore(gerbil_host_environment *snapshot) { (void)snapshot; }
#endif
#endif
