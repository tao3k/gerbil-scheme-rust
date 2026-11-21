"""Bounded long-lived reuse: post-GC native bytes and current RSS are distinct."""
import ctypes,json,os,subprocess,time
import gerbil_ledger

class MallocStats(ctypes.Structure):
    _fields_=[("blocks",ctypes.c_uint),("live",ctypes.c_size_t),("high_water",ctypes.c_size_t),("reserved",ctypes.c_size_t)]
_libc=ctypes.CDLL(None)
_libc.malloc_zone_statistics.argtypes=[ctypes.c_void_p,ctypes.POINTER(MallocStats)]
_libc.malloc_zone_pressure_relief.argtypes=[ctypes.c_void_p,ctypes.c_size_t]
_libc.malloc_zone_pressure_relief.restype=ctypes.c_size_t

def malloc_stats():
    stats=MallocStats();_libc.malloc_zone_statistics(None,ctypes.byref(stats))
    return dict(blocks=stats.blocks,live=stats.live,reserved=stats.reserved)

def rss_kib():
    return int(subprocess.check_output(["/bin/ps","-o","rss=","-p",str(os.getpid())],text=True))

def run():
    start=time.perf_counter_ns();library=gerbil_ledger.open_library()
    print(json.dumps(dict(kind="init",ns=time.perf_counter_ns()-start,states=library.vm_states)),flush=True)
    payload=b"x"*65536
    snapshots=[]
    for epoch in range(65):
        for batch in range(4):
            roots=library.call_many([("echo",payload) for _ in range(32)])
            for root in roots:root.close()
        if epoch%8==0:
            usage=library.collect();assert all(row[3]==0 for row in usage)
            snapshots.append(usage)
            print(json.dumps(dict(kind="reuse",epoch=epoch,calls=(epoch+1)*128,usage=usage,rss_kib=rss_kib(),malloc=malloc_stats())),flush=True)
    for owner in range(4):
        live=[rows[owner][2] for rows in snapshots[1:]]
        assert max(live)-min(live)<1024*1024,(owner,live)
    retained=library.call(("echo",payload));path=library.runtime_directory
    before=rss_kib();start=time.perf_counter_ns();library.close()
    assert library.shutdown_clean and library.shutdown_roots==[0]*4
    assert not os.path.exists(path)
    try:retained.to_python()
    except RuntimeError:pass
    else:raise AssertionError("closed owner retained a callable root")
    retained.close()
    print(json.dumps(dict(kind="close",ns=time.perf_counter_ns()-start,rss_before_kib=before,rss_after_kib=rss_kib(),malloc=malloc_stats(),roots=library.shutdown_roots,files_removed=True)),flush=True)

    released=_libc.malloc_zone_pressure_relief(None,0)
    print(json.dumps(dict(kind="diagnostic-pressure-relief",released=released,rss_kib=rss_kib(),malloc=malloc_stats())),flush=True)

if __name__=="__main__":run()
