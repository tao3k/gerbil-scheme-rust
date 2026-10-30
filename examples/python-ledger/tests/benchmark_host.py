"""Host observations outside timed native calls; never terminate unrelated work."""
import os
import subprocess


def host_snapshot():
    rows = []
    for line in subprocess.check_output(
        ["/bin/ps", "-axo", "pid=,pcpu=,comm="], text=True
    ).splitlines():
        pid, cpu, command = line.strip().split(None, 2)
        if int(pid) != os.getpid() and command.rsplit("/", 1)[-1] != "ps":
            if float(cpu) >= 20 or command.rsplit("/", 1)[-1] in {"gsc", "gerbil", "gxi", "gxtest", "cc1", "rustc", "clippy-driver", "lean"}:
                rows.append(dict(pid=int(pid), cpu=float(cpu), command=command))
    return rows

