(export gerbil_scheme_rust_abi_version
        gerbil_scheme_rust_runtime_init
        gerbil_scheme_rust_runtime_cleanup
        gerbil_scheme_rust_identity_i64
        gerbil_scheme_rust_add_i64
        gerbil_scheme_rust_is_even_i64
        gerbil_scheme_rust_compare_i64)

;; ASP-only source surface for the native Gerbil/Rust ABI exports.
;;
;; Keep this file out of the runtime build.  The implementation owner remains
;; scheme/native.ss; this file gives source-analysis tools ordinary Gerbil
;; definitions for ABI names whose implementation uses native FFI forms.

(def gerbil_scheme_rust_abi_version
  'native-abi-export)

(def gerbil_scheme_rust_runtime_init
  'native-abi-export)

(def gerbil_scheme_rust_runtime_cleanup
  'native-abi-export)

(def gerbil_scheme_rust_identity_i64
  'native-abi-export)

(def gerbil_scheme_rust_add_i64
  'native-abi-export)

(def gerbil_scheme_rust_is_even_i64
  'native-abi-export)

(def gerbil_scheme_rust_compare_i64
  'native-abi-export)
