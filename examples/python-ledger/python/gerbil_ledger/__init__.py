"""Independent Scheme Library using only the shared upstream Python bridge."""
from pathlib import Path
from ._core import _open


def open_library(*, backend="rust", owners=4, dispatch_workers=2):
    return _open(str(Path(__file__).parent), backend, owners,dispatch_workers)
