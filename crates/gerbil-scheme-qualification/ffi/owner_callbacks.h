/* Qualification-only foreign callbacks and TLS; no production Scheme policy. */
#include <pthread.h>
#include <stdint.h>
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
