/* Image-local SDK loader ownership. No host libc symbol is interposed. */
#include <dlfcn.h>
#include <os/lock.h>
#include <stdlib.h>
#include <stdint.h>
typedef int32_t (*gerbil_instance_close_fn)(void *);
static gerbil_instance_close_fn gerbil_system_close;
/* Darwin static lock has no per-image destructor registration. Its memory
 * remains mapped until actual join and all code leases/loader calls retire. */
static os_unfair_lock gerbil_module_lock = OS_UNFAIR_LOCK_INIT;
static uint64_t gerbil_code_leases;
static int gerbil_loader_retiring;
struct gerbil_retired_module { void *handle; struct gerbil_retired_module *next; };
static struct gerbil_retired_module *gerbil_modules;
static struct gerbil_retired_module *gerbil_modules_tail;
static uint64_t gerbil_module_count;
static uint64_t gerbil_module_capture_failures;
void gerbil_scheme_instance_bind_loader(gerbil_instance_close_fn close_function) {
  gerbil_system_close = close_function;
}
void gerbil_scheme_instance_code_leases(uint64_t count) {
  __atomic_store_n(&gerbil_code_leases, count, __ATOMIC_SEQ_CST);
}
/* The export allow-list hides this definition. Only this image's statically
 * linked SDK binds here. Constructors can still close non-VM loader handles. */
int dlclose(void *handle) {
  gerbil_instance_close_fn close_function = gerbil_system_close;
  if (!close_function) {
    close_function = (gerbil_instance_close_fn)dlsym(RTLD_DEFAULT, "dlclose");
    if (!close_function || close_function == dlclose) return -1;
  }
  if (!__atomic_load_n(&gerbil_loader_retiring, __ATOMIC_SEQ_CST)
      && __atomic_load_n(&gerbil_code_leases, __ATOMIC_SEQ_CST) == 0)
    return close_function(handle);
  /* Compiled only into the separate qualification fault image. */
#if defined(GERBIL_OWNER_CAPTURE_FAILURE) && GERBIL_OWNER_CAPTURE_FAILURE
  struct gerbil_retired_module *entry = NULL;
#else
  struct gerbil_retired_module *entry = malloc(sizeof(*entry));
#endif
  os_unfair_lock_lock(&gerbil_module_lock);
  if (!entry) {
    if (gerbil_module_capture_failures != UINT64_MAX) ++gerbil_module_capture_failures;
    os_unfair_lock_unlock(&gerbil_module_lock);
    return -1; /* Keep the loader reference; cleanup reports capture failure. */
  }
  entry->handle = handle;
  entry->next = NULL;
  if (gerbil_modules_tail) gerbil_modules_tail->next = entry;
  else gerbil_modules = entry;
  gerbil_modules_tail = entry;
  ++gerbil_module_count;
  os_unfair_lock_unlock(&gerbil_module_lock);
  return 0;
}
uint64_t gerbil_scheme_instance_pending_modules(void) {
  os_unfair_lock_lock(&gerbil_module_lock);
  uint64_t count = gerbil_module_count;
  uint64_t failed = gerbil_module_capture_failures;
  os_unfair_lock_unlock(&gerbil_module_lock);
  return failed > UINT64_MAX-count ? UINT64_MAX : count+failed;
}
/* Host must prove clean SDK cleanup, actual owner join and zero code leases.
 * Never hold the module lock across OS loader destructors. */
int32_t gerbil_scheme_instance_release_modules(void) {
  if (!gerbil_system_close || __atomic_load_n(&gerbil_code_leases, __ATOMIC_SEQ_CST)) return -1;
  for (;;) {
    os_unfair_lock_lock(&gerbil_module_lock);
    if (gerbil_module_capture_failures) { os_unfair_lock_unlock(&gerbil_module_lock); return -1; }
    struct gerbil_retired_module *entry = gerbil_modules;
    if (entry) {
      gerbil_modules = entry->next;
      if (!gerbil_modules) gerbil_modules_tail = NULL;
    }
    os_unfair_lock_unlock(&gerbil_module_lock);
    if (!entry) return 0;
    int status = gerbil_system_close(entry->handle);
    os_unfair_lock_lock(&gerbil_module_lock);
    if (status != 0) {
      entry->next = gerbil_modules;
      gerbil_modules = entry;
      if (!gerbil_modules_tail) gerbil_modules_tail = entry;
      os_unfair_lock_unlock(&gerbil_module_lock);
      return status;
    }
    --gerbil_module_count;
    os_unfair_lock_unlock(&gerbil_module_lock);
    free(entry);
  }
}
