
### Paired Gambit discovery

Runtime commands and native archive builds share Gambit compiler discovery.
Default selection follows the selected Gerbil executable, including symlinked
Homebrew wrappers with a literal `GERBIL_HOME`, before considering a validated
PATH compiler. Ghostscript's unrelated `gsc` is rejected. Both release versions
and development revision/timestamp version output are recognized. Explicit
`GERBIL_GSC` overrides remain supported and validated by native archive builds.
The selected installation supplies its own native runtime paths; downstream
CI does not need to set a second compiler override.
