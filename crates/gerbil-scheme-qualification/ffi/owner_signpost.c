/* Qualification-only clock marker; Apple's format builtin needs host Clang. */
#include <stdint.h>
#include <os/signpost.h>
#include <os/object.h>

void gerbil_owner_clock_signpost(uint64_t monotonic) {
  os_log_t log = os_log_create("org.gerbil-scheme-rust.qualification", "PointsOfInterest");
  os_signpost_event_emit(log, OS_SIGNPOST_ID_EXCLUSIVE, "OwnerClock",
    "monotonic_ns=%{public}llu", (unsigned long long)monotonic);
  os_release(log);
}
