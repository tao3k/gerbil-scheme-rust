"""Finite native VM reuse and lifecycle memory observations (Darwin).

Run each case in a fresh process. RSS and loaded images are observations,
not assertions of complete reclamation or indefinite leak freedom.
"""
import argparse
import concurrent.futures
import ctypes
import gc
import hashlib
import json
import math
import os
import resource
from pathlib import Path
import subprocess
import sys
import time

import gerbil_ledger

MODES = ("direct", "tokio", "rayon", "rayon-blocking")
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--case", choices=("reuse", "lifecycle"), required=True)
parser.add_argument("--seconds", type=float, default=300)
parser.add_argument("--cycles", type=int, default=8)
parser.add_argument("--owners", type=int, default=4)
parser.add_argument("--sample-seconds", type=float, default=5)
parser.add_argument("--max-rss-mib", type=float, default=2048)
parser.add_argument("--allocator-relief", action="store_true", help="diagnostic-only Darwin allocator relief after the final close")
a = parser.parse_args()
if sys.platform != "darwin":
    parser.error("Darwin observer required; no substitute image-count metric")
if any(not math.isfinite(x) for x in (a.seconds, a.sample_seconds, a.max_rss_mib)) or a.seconds < 0 or a.cycles < 1 or not 1 <= a.owners <= 8 or a.sample_seconds <= 0 or a.max_rss_mib <= 0:
    parser.error("positive cycles, interval, RSS bound; nonnegative duration; owners 1..8")
started = time.monotonic()
dyld = ctypes.CDLL(None)
dyld._dyld_image_count.restype = ctypes.c_uint32
dyld._dyld_get_image_name.argtypes = (ctypes.c_uint32,)
dyld._dyld_get_image_name.restype = ctypes.c_char_p
prefixes = (f"/gerbil-owners-{os.getpid()}-", f"/gerbil-python-modules-{os.getpid()}-")


def emit(kind, **row):
    print(json.dumps(dict(kind=kind, elapsed_seconds=time.monotonic()-started, **row)), flush=True)


def process():
    rss = int(subprocess.check_output(["/bin/ps", "-o", "rss=", "-p", str(os.getpid())], text=True, timeout=5).strip()) * 1024
    threads = len(subprocess.check_output(["/bin/ps", "-M", "-p", str(os.getpid())], text=True, timeout=5).splitlines()) - 1
    names = [dyld._dyld_get_image_name(i) for i in range(dyld._dyld_image_count())]
    owned = [name.decode() for name in names if name and any(p.encode() in name for p in prefixes)]
    descriptors = []
    for entry in os.listdir("/dev/fd"):
        try:
            os.fstat(int(entry))
        except (OSError, ValueError):
            continue
        descriptors.append(int(entry))
    return dict(rss_bytes=rss, os_threads=threads, open_fds=len(descriptors),
                fd_soft_limit=resource.getrlimit(resource.RLIMIT_NOFILE)[0],
                loaded_owner_images=sum(prefixes[0] in p for p in owned),
                loaded_module_images=sum(prefixes[1] in p for p in owned), loaded_images_with_existing_files=sum(Path(p).exists() for p in owned))


def observe(kind, library=None, expected_roots=None, **extra):
    gc.collect()
    usage = library.collect() if library is not None else None
    if expected_roots is not None:
        assert [row[3] for row in usage] == expected_roots, usage
    state = process()
    emit(kind, usage=usage, **state, **extra)
    assert state["rss_bytes"] <= a.max_rss_mib * 1024**2, "RSS resource guard exceeded; observation retained"
    return state


def close(library, initial_threads):
    path = library.runtime_directory
    library.close()
    assert library.shutdown_clean and library.shutdown_roots == [0] * a.owners
    assert not Path(path).exists()
    deadline = time.monotonic()+5
    while process()["os_threads"] > initial_threads and time.monotonic() < deadline:
        time.sleep(.01)
    assert process()["os_threads"] <= initial_threads, "owner/dispatch threads remain"
    # Check retirement before a second close can mask late resource creation.
    library.close()


def setup(library):
    sessions = [library.session(i) for i in range(a.owners)]
    for session in sessions:
        root = session.call(("dsl", "(let ((foldl #f)) (ledger-sum 20 22))"))
        assert root.to_python() == 42
        root.close()
    observe("warm-unrooted", library, [0] * a.owners)
    payload = tuple((i, tuple((j, 3) for j in range(32)), False) for i in range(16))
    retained = [(session.value(payload), session.call(("saved-continuation",)), session.call(("procedure",))) for session in sessions]
    observe("warm-rooted", library, [3] * a.owners)
    return sessions, retained


def work(session, roots, mode, turn):
    payload, continuation, procedure = roots
    values = session.call_many([("actors", payload, 1)] * 8, dispatch=mode, admission_window=4)
    rows = library.project_many(values, release=True)
    for row in rows:
        assert [v[0] for v in row] == list(range(16))
        assert {v[1] for v in row} == {3*32*31//2}
        assert sorted(v[2] for v in row) == list(range(1, 17))
        assert [sum(v[3] == worker for v in row) for worker in range(4)] == [4]*4
    value = continuation.apply(turn)
    assert value.to_python() == 65536+turn
    value.close()
    value = procedure.apply(turn)
    assert value.to_python() == turn+1
    value.close()
    data = (turn, 2**180, b"\0\xff" * 4096)
    values = session.call_many([("echo", data)] * 4, dispatch=mode, admission_window=4)
    assert library.project_many(values, release=True) == [list(data)] * 4
    if turn % 8 == 0:
        # Accepted result roots preceding/following a native exception must retire.
        try:
            session.call_many([("echo", turn), ("append", "not-an-integer"), ("echo", turn)], dispatch=mode)
        except RuntimeError:
            pass
        else:
            raise AssertionError("native Scheme exception not propagated")
        try:
            value = session.call(("dsl", "(+ 20 22)"))
        except RuntimeError as error:
            emit("dsl-recovery-failure", owner=payload.owner, turn=turn, mode=mode, error=str(error))
            raise
        assert value.to_python() == 42
        value.close()


emit("configuration", case=a.case, seconds=a.seconds, cycles=a.cycles, owners=a.owners,
     harness_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
     contract="native types; same fixed VMs per reuse case; mode rotation; explicit roots; finite RSS guard; SDK live metric raw units")
initial = observe("process-start")
completed = 0
try:
    for cycle in range(1 if a.case == "reuse" else a.cycles):
        library = gerbil_ledger.open_library(owners=a.owners, dispatch_workers=2)
        try:
            sessions, retained = setup(library)
            states = library.vm_states
            # A known extra root and allocation verify that root/live diagnostics
            # respond to retained data; release does not imply RSS must decrease.
            pulse = sessions[0].value(b"x" * (4*1024**2))
            observe("allocation-held", library, [4]+[3]*(a.owners-1), cycle=cycle)
            pulse.close()
            observe("allocation-released", library, [3]*a.owners, cycle=cycle)
            end = time.monotonic() + (a.seconds if a.case == "reuse" else 0)
            next_sample = time.monotonic()
            turn = 0
            with concurrent.futures.ThreadPoolExecutor(max_workers=a.owners) as pool:
                while turn < len(MODES) or time.monotonic() < end:
                    mode = MODES[turn % len(MODES)]
                    futures = [pool.submit(work, s, r, mode, turn) for s, r in zip(sessions, retained)]
                    for future in futures:
                        future.result(timeout=20)
                    turn += 1
                    completed += a.owners * 12
                    if time.monotonic() >= next_sample:
                        observe("reuse-sample", library, [3]*a.owners, cycle=cycle, turns=turn, completed_batch_requests=completed, mode=mode)
                        assert library.vm_states == states
                        next_sample = time.monotonic()+a.sample_seconds
            for roots in retained:
                for root in roots:
                    root.close()
            observe("released-all", library, [0]*a.owners, cycle=cycle, turns=turn)
            close(library, initial["os_threads"])
            observe("closed", cycle=cycle, roots=library.shutdown_roots, files_removed=True)
        finally:
            library.close()
        del sessions, retained, library
        gc.collect()
    if a.allocator_relief:
        before = observe("allocator-before")
        # Darwin's installed malloc.h specifies NULL = all zones, goal 0 =
        # maximal best-effort relief. This is not a production runtime policy.
        relief = dyld.malloc_zone_pressure_relief
        relief.argtypes = (ctypes.c_void_p, ctypes.c_size_t)
        relief.restype = ctypes.c_size_t
        released = relief(None, 0)
        after = observe("allocator-after", allocator_reported_released_bytes=released)
        assert before["loaded_owner_images"] == after["loaded_owner_images"]
    emit("summary", status="PASS", completed_batch_requests=completed, **process())
except BaseException as error:
    emit("failure", error=repr(error), **process())
    raise
