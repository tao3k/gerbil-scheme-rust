/* Process-local region diagnostics; never initializes or inspects a dead VM. */
#include <mach/mach_vm.h>
#include <mach/vm_statistics.h>
#include <mach-o/loader.h>
#include <sys/mman.h>
static int owner_malloc_tag(unsigned tag) {
  switch (tag) {
    case VM_MEMORY_MALLOC: case VM_MEMORY_MALLOC_SMALL:
    case VM_MEMORY_MALLOC_LARGE: case VM_MEMORY_MALLOC_HUGE:
    case VM_MEMORY_REALLOC: case VM_MEMORY_MALLOC_TINY:
    case VM_MEMORY_MALLOC_LARGE_REUSABLE: case VM_MEMORY_MALLOC_LARGE_REUSED:
    case VM_MEMORY_MALLOC_NANO: case VM_MEMORY_MALLOC_MEDIUM:
    case VM_MEMORY_MALLOC_PROB_GUARD: return 1;
    default: return 0;
  }
}
static int owner_segment_pages(uintptr_t address, uint64_t length,
                               uint64_t *resident, uint64_t *modified) {
  size_t page = (size_t)getpagesize();
  for (uint64_t offset = 0; offset < length;) {
    unsigned char states[256];
    size_t bytes = length - offset < sizeof(states) * page
      ? (size_t)(length - offset) : sizeof(states) * page;
    if (mincore((void *)(address + offset), bytes, (char *)states) != 0) return -1;
    size_t pages = (bytes + page - 1) / page;
    for (size_t i=0; i<pages; ++i) {
      if (states[i] & MINCORE_INCORE) *resident += page;
      if (states[i] & MINCORE_MODIFIED) *modified += page;
    }
    offset += bytes;
  }
  return 0;
}
/* malloc: virtual,resident,dirtied,reusable; matching images: text virtual and
 * resident, data virtual and resident, modified data, count, total virtual and
 * resident. Prefix is the exact process-owned copy directory prefix. */
int32_t gerbil_owner_memory_regions(const char *prefix, uint64_t *out) {
  memset(out, 0, 12 * sizeof(uint64_t));
  mach_vm_address_t address = 0;
  uint64_t page = (uint64_t)getpagesize();
  for (;;) {
    mach_vm_size_t size = 0;
    vm_region_extended_info_data_t info;
    mach_msg_type_number_t count = VM_REGION_EXTENDED_INFO_COUNT;
    mach_port_t object = MACH_PORT_NULL;
    kern_return_t status = mach_vm_region(mach_task_self(), &address, &size,
      VM_REGION_EXTENDED_INFO, (vm_region_info_t)&info, &count, &object);
    if (object != MACH_PORT_NULL) mach_port_deallocate(mach_task_self(), object);
    if (status == KERN_INVALID_ADDRESS) break;
    if (status != KERN_SUCCESS || size == 0) return -1;
    if (owner_malloc_tag(info.user_tag)) {
      out[0] += size; out[1] += info.pages_resident * page;
      out[2] += info.pages_dirtied * page; out[3] += info.pages_reusable * page;
    }
    if (address + size < address) break;
    address += size;
  }
  for (uint32_t i=0; i<_dyld_image_count(); ++i) {
    const char *name = _dyld_get_image_name(i);
    if (!name || strncmp(name, prefix, strlen(prefix))) continue;
    const struct mach_header_64 *header = (const struct mach_header_64 *)_dyld_get_image_header(i);
    if (header->magic != MH_MAGIC_64) return -2;
    const struct load_command *command = (const struct load_command *)(header + 1);
    intptr_t slide = _dyld_get_image_vmaddr_slide(i);
    ++out[9];
    for (uint32_t j=0; j<header->ncmds; ++j) {
      if (command->cmd == LC_SEGMENT_64) {
        const struct segment_command_64 *segment = (const struct segment_command_64 *)command;
        uint64_t resident = 0, modified = 0;
        if (owner_segment_pages((uintptr_t)(slide + segment->vmaddr), segment->vmsize,
                                &resident, &modified)) return -3;
        out[10] += segment->vmsize; out[11] += resident;
        if (!strcmp(segment->segname, "__TEXT")) {
          out[4] += segment->vmsize; out[5] += resident;
        } else if (!strncmp(segment->segname, "__DATA", 6)) {
          out[6] += segment->vmsize; out[7] += resident; out[8] += modified;
        }
      }
      command = (const struct load_command *)((const char *)command + command->cmdsize);
    }
  }
  return 0;
}

#include <pthread.h>
#include <errno.h>
#include <limits.h>
/* Quiescent diagnostic: reserve then release all currently available TLS keys. */
int32_t gerbil_owner_tls_available(void) {
  pthread_key_t keys[PTHREAD_KEYS_MAX];
  int count = 0, status = 0;
  while (count < PTHREAD_KEYS_MAX) {
    status = pthread_key_create(&keys[count], NULL);
    if (status != 0) break;
    ++count;
  }
  for (int i=0; i<count; ++i) if (pthread_key_delete(keys[i]) != 0) return -1;
  return status == 0 || status == EAGAIN ? count : -2;
}
