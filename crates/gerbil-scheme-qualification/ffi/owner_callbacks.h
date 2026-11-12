/* Qualification-only foreign callbacks and TLS; no production Scheme policy. */
#include <pthread.h>
#include <stdint.h>
#include <time.h>
static pthread_key_t owner_callback_key;
static int owner_callback_key_live = 0;
static void owner_callback_destructor(void *counter) {
  __atomic_fetch_add((uint64_t *)counter, 1, __ATOMIC_SEQ_CST);
}
int32_t gerbil_owner_callback_tls_start(uint64_t *counter) {
  if (owner_callback_key_live || counter == NULL) return -1;
  int status = pthread_key_create(&owner_callback_key, owner_callback_destructor);
  if (status != 0) return status;
  owner_callback_key_live = 1;
  return pthread_setspecific(owner_callback_key, counter);
}
int32_t gerbil_owner_callback_tls_retire(void) {
  if (!owner_callback_key_live) return -1;
  int status = pthread_key_delete(owner_callback_key);
  if (status == 0) owner_callback_key_live = 0;
  return status;
}
uint64_t gerbil_owner_callback(uint64_t value) { return value + 1; }
int64_t gerbil_owner_failed_finalizer(int64_t input) { (void)input; return 0; }
int32_t gerbil_owner_runtime_state(void) { return gerbil_scheme_rust_state; }

static pthread_key_t owner_blocking_key;
static void owner_blocking_destructor(void *pointer) {
  uint64_t *flags = pointer;
  __atomic_store_n(&flags[0], 1, __ATOMIC_SEQ_CST);
  const struct timespec interval = {0, 1000000};
  while (__atomic_load_n(&flags[1], __ATOMIC_SEQ_CST) == 0)
    nanosleep(&interval, NULL);
}
int32_t gerbil_owner_blocking_tls_start(uint64_t *flags) {
  int status = pthread_key_create(&owner_blocking_key, owner_blocking_destructor);
  if (status != 0) return status;
  return pthread_setspecific(owner_blocking_key, flags);
}
int32_t gerbil_owner_blocking_tls_retire(void) {
  return pthread_key_delete(owner_blocking_key);
}

/* Pure C callback: acquire one loader reference, then use the image-local SDK
 * close adapter. A live code lease forces the owned reference into retirement. */
int32_t gerbil_owner_loader_roundtrip(const char *path) {
  void *handle = dlopen(path, RTLD_NOW | RTLD_LOCAL | RTLD_NOLOAD);
  if (!handle) return -2;
  return dlclose(handle);
}
