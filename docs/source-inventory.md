# Frozen Source Inventory

`stage_source_inventory` publishes source evidence as inert data, not installed
modules. Each declared original path becomes a directory with a `source.bin`
payload. Every input keeps its own exact SHA and original diagnostic origin.
Neither an aggregate digest nor a runtime SDK path replaces those bindings.

This layout prevents ordinary Python installers from compiling archived `.py`
files and adding undeclared bytecode inside the verified source root. Runtime
resource staging is separate: `stage_resources` and module closures retain their
actual loader-visible names. Source evidence is never a module load path.

Verification still rejects every undeclared file, symlink, path escape, missing
input and byte drift. It does not ignore `__pycache__`, delete input evidence, or
require callers to disable ordinary installer behavior. Failed staging is retained;
only successfully verified temporary staging is cleaned.

Tests exercise the real Python byte compiler, payload-name collisions, complete
source counts, unchanged source bytes and undeclared-file rejection. Installed
wheel qualification remains an independent consumer obligation.
