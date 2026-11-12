"""Explicit retirement qualification; production open_library keeps its policy.

Use only for an owned workload that quiesces VM callbacks before close and holds
code leases for escaping native callbacks/TLS. This provider assertion does not
limit Scheme evaluation or grant permission to enter an already closed VM.
"""
from pathlib import Path
from ._core import _open_qualified_retirement


def open_qualified_retirement_library(*, owners=4, dispatch_workers=2):
    return _open_qualified_retirement(str(Path(__file__).parent), owners, dispatch_workers)
