/* Qualification adds observations to the shared native lifecycle adapter. */
#define GERBIL_INSTANCE_LINKER ___LNK_gerbil__actor__qualification
#include "../../../ffi/instance.c"
#include <mach/mach.h>
#include <mach-o/dyld.h>
#include <libproc.h>
#include <stdlib.h>
#include <unistd.h>
#include <malloc/malloc.h>
/* Current resources, not getrusage's historical RSS high-water mark. */
int32_t gerbil_owner_process_resources(uint64_t *out) {
  mach_task_basic_info_data_t basic;
  mach_msg_type_number_t basic_count = MACH_TASK_BASIC_INFO_COUNT;
  task_vm_info_data_t vm;
  mach_msg_type_number_t vm_count = TASK_VM_INFO_COUNT;
  if (task_info(mach_task_self(), MACH_TASK_BASIC_INFO, (task_info_t)&basic, &basic_count) != KERN_SUCCESS) return -1;
  if (task_info(mach_task_self(), TASK_VM_INFO, (task_info_t)&vm, &vm_count) != KERN_SUCCESS) return -2;
  out[0] = basic.resident_size;
  out[1] = vm.phys_footprint;
  int capacity = proc_pidinfo(getpid(), PROC_PIDLISTFDS, 0, NULL, 0);
  if (capacity <= 0) return -3;
  void *fds = malloc((size_t)capacity);
  if (!fds) return -4;
  int used = proc_pidinfo(getpid(), PROC_PIDLISTFDS, 0, fds, capacity);
  free(fds);
  if (used < 0) return -5;
  out[2] = (uint64_t)used / sizeof(struct proc_fdinfo);
  thread_act_array_t threads;
  mach_msg_type_number_t count;
  if (task_threads(mach_task_self(), &threads, &count) != KERN_SUCCESS) return -6;
  out[3] = count;
  for (mach_msg_type_number_t i=0; i<count; ++i) mach_port_deallocate(mach_task_self(), threads[i]);
  vm_deallocate(mach_task_self(), (vm_address_t)threads, count * sizeof(thread_t));
  out[4] = _dyld_image_count();
  malloc_statistics_t allocation;
  malloc_zone_statistics(NULL, &allocation);
  out[5] = allocation.size_in_use;
  out[6] = allocation.size_allocated;
  out[7] = allocation.blocks_in_use;
  return 0;
}

uint64_t gerbil_owner_allocator_relief(void) {
  return (uint64_t)malloc_zone_pressure_relief(NULL, 0);
}

#include "owner_memory.h"


#include "owner_callbacks.h"

#include "owner_wait.h"
