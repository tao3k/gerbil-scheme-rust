"""Installed native macro workload; compare wheels without compiling in the timed path."""
import argparse
import json
import math
import os
import statistics
import time

import gerbil_ledger
from benchmark_host import host_snapshot


def percentile(values, p):
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * p) - 1)]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--label", required=True)
    parser.add_argument("--owners", type=int, default=4)
    parser.add_argument("--iterations", type=int, default=32)
    parser.add_argument("--rounds", type=int, default=8)
    parser.add_argument("--require-quiet", action="store_true")
    args = parser.parse_args()
    if min(args.owners, args.iterations, args.rounds) < 1:
        parser.error("counts must be positive")
    contaminated = False
    def emit(record):
        nonlocal contaminated
        contaminated |= any(record.get(key) for key in
                            ("host_busy", "host_busy_before", "host_busy_after"))
        print(json.dumps(dict(label=args.label, **record)), flush=True)
    busy = host_snapshot()
    if args.require_quiet and busy:
        emit(dict(kind="blocked", reason="host is busy", host_busy=busy))
        raise SystemExit(75)
    emit(dict(kind="configuration", owners=args.owners, iterations=args.iterations,
              rounds=args.rounds, require_quiet=args.require_quiet, host_busy=busy,
              timing_contract="API-ready; projected-and-released response; 32-operation batches by default"))
    start = time.perf_counter_ns()
    library = gerbil_ledger.open_library(owners=args.owners)
    emit(dict(kind="open", ns=time.perf_counter_ns() - start,
              states=library.vm_states, host_busy=host_snapshot()))
    # Shadowing foldl verifies the macro definition's binding survives expansion.
    source = "(let ((foldl #f)) (ledger-sum 20 22))"
    samples = {"macro-expand": [], "macro-eval": []}
    responses = {mode: [] for mode in samples}
    try:
        for owner in range(args.owners):
            start = time.perf_counter_ns()
            root = library.call_on(owner, ("dsl", source))
            assert root.to_python() == 42
            root.close()
            emit(dict(kind="cold-dsl", owner=owner, ns=time.perf_counter_ns()-start,
                      host_busy=host_snapshot()))
        for index in range(args.rounds + 2):
            modes = ["macro-expand", "macro-eval"]
            if index % 2:
                modes.reverse()
            for mode in modes:
                before = host_snapshot()
                for owner in range(args.owners):
                    start = time.perf_counter_ns()
                    root = library.call_on(owner, (mode, source, args.iterations))
                    native = time.perf_counter_ns()
                    value = root.to_python()
                    root.close()
                    projected = time.perf_counter_ns()
                    assert value == [args.iterations, 42]
                    sample = native-start
                    if index >= 2:
                        samples[mode].append(sample)
                        responses[mode].append(projected-start)
                    emit(dict(kind="sample", round=index, warmup=index<2, mode=mode,
                              owner=owner, iterations=args.iterations, ns=sample,
                              projection_release_ns=projected-native,
                              response_ns=projected-start,
                              host_busy_before=before, host_busy_after=host_snapshot()))
        emit(dict(kind="gc", usage=library.collect()))
        directory = library.runtime_directory
        library.close()
        assert library.shutdown_clean and library.shutdown_roots == [0]*args.owners
        assert not os.path.exists(directory)
        emit(dict(kind="summary", summary={mode: dict(
            samples=len(values), iterations=args.iterations,
            median_ns=statistics.median(values), p99_ns=percentile(values, .99),
            response_p99_ns=percentile(responses[mode], .99),
            expansions_per_second=len(values)*args.iterations/(sum(values)/1e9),
            projected_expansions_per_second=len(values)*args.iterations/(sum(responses[mode])/1e9)
        ) for mode, values in samples.items()}, roots=library.shutdown_roots,
            quiet_snapshots=not contaminated,
            admission=("REJECTED_HOST_BUSY" if contaminated else "QUIET_SNAPSHOTS")))
        return 75 if args.require_quiet and contaminated else 0
    except BaseException as error:
        emit(dict(kind="failure", error=str(error)))
        try:
            library.close()
        except BaseException as cleanup:
            emit(dict(kind="shutdown-failure", error=str(cleanup)))
        os._exit(1)


if __name__ == "__main__":
    raise SystemExit(main())
