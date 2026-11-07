# Complete Package Verification

`gerbil-scheme-python` offers the explicit Cargo feature
`parallel-verification`. It parallelizes independent directory branches using
Rayon, without changing the `PackageManifest::verify` contract. A caller without
the feature uses serial verification; this is a build-time choice, not runtime
fallback after a failed verification.

Both implementations use the same manifest validation and leaf-file inspector.
Every call reads the complete declared inventory and every file's SHA256.
Missing or extra files, aliases (including parent symlinks), unsafe paths,
special files, changed bytes and unknown schema fields remain errors. Parallel
branches own their observed-path sets; they publish the combined verified owner
only after all branches finish successfully. A failure never returns a partial
`VerifiedPackage`. Results are not cached validation verdicts.

`RAYON_NUM_THREADS` controls the global Rayon pool before its first use. Compare
installed libraries using identical bytes, complete manifests and alternating
fresh processes. Small file sets can be slower under parallel scheduling;
feature availability alone establishes no speedup. Build time, package
verification, DSL initialization, inference and teardown are distinct phases.
