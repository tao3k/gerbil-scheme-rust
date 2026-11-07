"""Fresh-process invalid startup must not enter Scheme or leave runtime files."""
import json
import os
from pathlib import Path
import tempfile

import gerbil_ledger
from gerbil_ledger._core import scheme_initialization_attempted

def namespaces():
    return set(Path(tempfile.gettempdir()).glob(f"gerbil-owners-{os.getpid()}-*"))

assert scheme_initialization_attempted() is False
before=namespaces()
for options in ({"owners":0},{"owners":65},{"dispatch_workers":0},{"dispatch_workers":65}):
    try:gerbil_ledger.open_library(**options)
    except RuntimeError:pass
    else:raise AssertionError(f"invalid startup admitted: {options}")
    assert scheme_initialization_attempted() is False
    assert namespaces()==before
library=gerbil_ledger.open_library(owners=1,dispatch_workers=1)
try:
    assert scheme_initialization_attempted() is True and library.vm_count==1 and library.active_processors==1
    continuation=library.session(0).call(("saved-continuation",))
    library.collect()
    for value in (1,42):
        root=continuation.apply(value);assert root.to_python()==65536+value;root.close()
    continuation.close()
    path=library.runtime_directory;library.close()
    assert library.shutdown_clean and library.shutdown_roots==[0] and not os.path.exists(path)
    assert namespaces()==before
    print(json.dumps(dict(status="PASS",invalid_configurations=4,entered_before_valid_start=False,continuations_after_gc=True,roots=library.shutdown_roots,files_removed=True)),flush=True)
finally:
    library.close()
