(export gerbil_scheme_rust_abi_version
         gerbil_scheme_rust_runtime_init
         gerbil_scheme_rust_runtime_cleanup
         gerbil_scheme_rust_identity_i64
         gerbil_scheme_rust_add_i64
         gerbil_scheme_rust_is_even_i64
         gerbil_scheme_rust_compare_i64
         gerbil_scheme_rust_runtime_handle_shape
         gerbil_scheme_rust_status_shape
         gerbil_scheme_rust_i64_shape
         gerbil_scheme_rust_bool_shape
         gerbil_scheme_rust_comparison_shape
         gerbil_scheme_rust_native_value_shape
         gerbil_scheme_rust_native_result_shape)

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

(def gerbil_scheme_rust_runtime_handle_shape
  '(native-shape
    (name . gerbil-runtime-handle)
    (ownership . rust-borrowed-process-global)
    (thread-affinity . initializing-thread)
    (send . false)
    (sync . false)))

(def gerbil_scheme_rust_status_shape
  '(native-shape
    (name . gerbil-status)
    (repr . c-int)
    (variants
     (ok . 0)
     (runtime-init-failed . 1)
     (module-init-failed . 2)
     (abi-version-mismatch . 3)
     (already-initialized . 4)
     (not-initialized . 5)
     (wrong-thread . 6)
     (invalid-value . 7))))

(def gerbil_scheme_rust_i64_shape
  '(native-shape
    (name . i64)
    (repr . signed-64-bit-integer)
    (ownership . by-value)
    (range . full-i64)))

(def gerbil_scheme_rust_bool_shape
  '(native-shape
    (name . bool)
    (repr . c-int)
    (false . 0)
    (true . non-zero)))

(def gerbil_scheme_rust_comparison_shape
  '(native-shape
    (name . comparison)
    (repr . c-int)
    (less . -1)
    (equal . 0)
    (greater . 1)))

(def gerbil_scheme_rust_utf8_shape
  '(native-shape
    (name . utf8)
    (transport . c-abi)
    (repr . borrowed-byte-slice)
    (encoding . utf-8)
    (ptr . const-char-pointer)
    (len . size-t)
    (ownership . rust-borrowed)
    (nullability . empty-allows-null-pointer)
    (lifetime . caller-bounded)))

(def gerbil_scheme_rust_value_handle_shape
  '(native-shape
    (name . gerbil-value-handle)
    (transport . c-abi)
    (repr . opaque-pointer)
    (ownership . gerbil-runtime-owned)
    (nullability . non-null)
    (dereference-policy . never-deref-in-rust-safe-layer)
    (rooting . unrooted-borrow)
    (gc-policy . no-gc-root-guarantee)))

(def gerbil_scheme_rust_i64_callback_shape
  '(native-shape
    (name . i64-callback)
    (transport . c-abi)
    (repr . function-pointer-plus-context)
    (input . i64)
    (return . gerbil-status)
    (context-nullability . non-null)
    (panic-policy . contained-as-panic-status)
    (ownership . rust-owned-callback-context)))

(def gerbil_scheme_rust_native_value_shape
  '(native-shape
    (name . native-value)
    (transport . c-abi)
    (scalar-values (i64 bool comparison status))
    (borrowed-values (utf8))
    (handle-values (runtime-handle gerbil-value-handle))
    (callback-values (i64-callback))
    (nullability . explicit-per-shape)
    (rooting . explicit-per-shape)))

(def gerbil_scheme_rust_native_result_shape
  '(native-shape
    (name . native-result)
    (ok . native-value)
    (error . gerbil-status)
    (failure-policy . fail-closed)))
