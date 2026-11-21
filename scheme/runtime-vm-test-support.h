/* SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later */
#include <pthread.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>

extern void ___thread_set_pstate(___processor_state);
extern int gerbil_rust_vm_test_exercise(int);
extern int gerbil_rust_vm_test_retained(int);

typedef struct {
  uintptr_t vm;
  uintptr_t processor;
  int created;
  int selected;
  int processors;
  int outcome;
} gerbil_vm_test_slot;

static gerbil_vm_test_slot gerbil_vm_slots[2];
static pthread_mutex_t gerbil_vm_lifecycle = PTHREAD_MUTEX_INITIALIZER;
static atomic_int gerbil_vm_created, gerbil_vm_entered, gerbil_vm_closed;
static _Thread_local int gerbil_vm_role;
static int gerbil_vm_invoked;

static int gerbil_vm_wait(atomic_int *counter, int target) {
  struct timespec start, now;
  clock_gettime(CLOCK_MONOTONIC, &start);
  while (atomic_load(counter) < target) {
    clock_gettime(CLOCK_MONOTONIC, &now);
    if (now.tv_sec - start.tv_sec >= 3) return 0;
    usleep(1000);
  }
  return 1;
}

static int gerbil_vm_overlap(void) {
  atomic_fetch_add(&gerbil_vm_entered, 1);
  return gerbil_vm_wait(&gerbil_vm_entered, 2);
}

static void *gerbil_vm_owner(void *argument) {
  int role = (int)(intptr_t)argument;
  gerbil_vm_test_slot *slot = &gerbil_vm_slots[role - 1];
  gerbil_vm_role = role;
  ___virtual_machine_state vm = calloc(1, sizeof(*vm));
  if (!vm) { slot->outcome = 10; return NULL; }
  pthread_mutex_lock(&gerbil_vm_lifecycle);
  ___SCMOBJ error = ___setup_vmstate(vm);
  pthread_mutex_unlock(&gerbil_vm_lifecycle);
  if (error != ___FIX(___NO_ERR)) {
    /* Partial constructor rollback is not guessed. The watchdog child exits. */
    fprintf(stderr, "VM constructor failed role=%d error=%ld\n", role, (long)error);
    _Exit(11);
  }
  ___processor_state ps = ___PSTATE_FROM_PROCESSOR_ID(0, vm);
  slot->vm = (uintptr_t)vm;
  slot->processor = (uintptr_t)ps;
  slot->created = 1;
  slot->processors = vm->processor_count;
  atomic_fetch_add(&gerbil_vm_created, 1);
  if (!gerbil_vm_wait(&gerbil_vm_created, 2)) {
    slot->outcome = 12;
    goto cleanup;
  }
  ___thread_set_pstate(ps);
  slot->selected = ___PSTATE == ps;
  if (!slot->selected || vm->processor_count != 1) {
    slot->outcome = 13;
    goto cleanup;
  }
#ifndef ___SINGLE_THREADED_VMS
  error = ___GSTATE->___thread_init_from_self(&ps->os_thread);
  if (error != ___FIX(___NO_ERR)) { slot->outcome = 14; goto cleanup; }
#endif
  vm->main_module_ref = ___GSTATE->vmstate0.main_module_ref;
  ___SCMOBJ kernel = ___VECTORELEM(
    ___VECTORELEM(___VECTORELEM(___GSTATE->program_descr, 0), 0),
    ___MODULE_DESCR_THUNK);
  error = ___run(kernel);
  if (error != ___FIX(___NO_ERR)) { slot->outcome = 15; goto cleanup; }
  if (gerbil_rust_vm_test_exercise(role) != role) { slot->outcome = 16; goto cleanup; }
  if (role == 2) {
    if (!gerbil_vm_wait(&gerbil_vm_closed, 1) ||
        gerbil_rust_vm_test_retained(role) != role) {
      slot->outcome = 17;
      goto cleanup;
    }
  }
  slot->outcome = 0;
cleanup:
  pthread_mutex_lock(&gerbil_vm_lifecycle);
  ___cleanup_pstate(ps);
  ___MUTEX_LOCK(___GSTATE->vm_list_mut);
  ___cleanup_vmstate(vm);
  ___MUTEX_UNLOCK(___GSTATE->vm_list_mut);
  pthread_mutex_unlock(&gerbil_vm_lifecycle);
  free(vm);
  if (role == 1) atomic_store(&gerbil_vm_closed, 1);
  return NULL;
}

static int gerbil_vm_test_run(void) {
  if (gerbil_vm_invoked++) return 20;
  pthread_t threads[2];
  for (int i = 0; i < 2; ++i)
    if (pthread_create(&threads[i], NULL, gerbil_vm_owner, (void *)(intptr_t)(i + 1)))
      _Exit(21);
  for (int i = 0; i < 2; ++i)
    if (pthread_join(threads[i], NULL)) _Exit(22);
  int valid = atomic_load(&gerbil_vm_entered) == 2 &&
              gerbil_vm_slots[0].vm != gerbil_vm_slots[1].vm;
  for (int i = 0; i < 2; ++i) {
    gerbil_vm_test_slot *slot = &gerbil_vm_slots[i];
    fprintf(stderr, "VM-TEST owner=%d vm=%p processor0=%p created=%d selected=%d processors=%d outcome=%d\n",
      i + 1, (void *)slot->vm, (void *)slot->processor,
      slot->created, slot->selected, slot->processors, slot->outcome);
    valid = valid && slot->created && slot->selected &&
            slot->processors == 1 && slot->outcome == 0;
  }
  return valid ? 0 : 23;
}
