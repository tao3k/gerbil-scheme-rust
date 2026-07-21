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
         gerbil_scheme_rust_fixnum_shape
         gerbil_scheme_rust_char_shape
         gerbil_scheme_rust_flonum_shape
         gerbil_scheme_rust_bytevector_shape
         gerbil_scheme_rust_rooted_bytes_shape
         gerbil_scheme_rust_utf8_shape
         gerbil_scheme_rust_value_handle_shape
         gerbil_scheme_rust_nil_shape
         gerbil_scheme_rust_void_shape
         gerbil_scheme_rust_i64_callback_shape
         gerbil_scheme_rust_native_value_shape
         gerbil_scheme_rust_native_error_shape
         gerbil_scheme_rust_native_result_shape)

;; ASP-only source-analysis projection for the native Gerbil/Rust ABI surface.
;;
;; Keep this file out of the runtime build.  The implementation owner remains
;; scheme/native.ss and the tracked Gerbil contract is scheme/native.ssi.  This
;; file gives source-analysis tools ordinary Gerbil definitions for ABI names
;; and value-family shapes whose implementation uses native FFI forms.
;;
;; Removal criterion: retire this projection when the Gerbil provider can
;; directly project scheme/native.ss plus scheme/native.ssi FFI exports.

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

(def gerbil_scheme_rust_fixnum_shape
  '(native-shape
    (name . fixnum)
    (transport . c-abi)
    (repr . machine-word)
    (ownership . by-value-or-scheme-object-export)
    (predicate . gerbil-rs-scheme-object-fixnum?-raw)
    (projection . gerbil-rs-scheme-object-fixnum-value-raw)
    (safe-methods (is-fixnum as-fixnum as-fixnum-i64))
    (failure-policy . fail-closed)))

(def gerbil_scheme_rust_char_shape
  '(native-shape
    (name . char)
    (transport . c-abi)
    (repr . unicode-scalar-u32)
    (ownership . by-value-or-scheme-object-export)
    (predicate . gerbil-rs-scheme-object-char?-raw)
    (projection . gerbil-rs-scheme-object-char-value-raw)
    (fixtures (ascii bmp non-bmp))
    (safe-methods (is-char as-char))
    (validation . rust-unicode-scalar)
    (failure-policy . fail-closed)))

(def gerbil_scheme_rust_flonum_shape
  '(native-shape
    (name . flonum)
    (transport . c-abi)
    (repr . ieee-754-f64)
    (ownership . by-value-or-scheme-object-export)
    (predicate . gerbil-rs-scheme-object-flonum?-raw)
    (projection . gerbil-rs-scheme-object-flonum-value-raw)
    (fixtures (finite nan positive-infinity negative-infinity negative-zero))
    (safe-methods (is-flonum as-flonum))
    (nan-policy . preserve-rust-is-nan)
    (infinity-policy . preserve-sign)
    (zero-policy . preserve-sign)
    (failure-policy . fail-closed)))

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

(def gerbil_scheme_rust_nil_shape
  '(native-shape
    (name . nil)
    (transport . c-abi)
    (repr . gerbil-value-handle)
    (ownership . gerbil-runtime-owned)
    (nullability . non-zero-handle)
    (sentinel . empty-list)
    (predicate . null?)
    (projection . SchemeNil)
    (rooting . unrooted-borrow)
    (gc-policy . no-gc-root-guarantee)))

(def gerbil_scheme_rust_void_shape
  '(native-shape
    (name . void)
    (transport . c-abi)
    (repr . gerbil-value-handle)
    (ownership . gerbil-runtime-owned)
    (nullability . non-zero-handle)
    (sentinel . void)
    (predicate . eq?-#!void)
    (projection . SchemeVoid)
    (rooting . unrooted-borrow)
    (gc-policy . no-gc-root-guarantee)))

(def gerbil_scheme_rust_bytevector_shape
  '(native-shape
    (name . bytevector)
    (transport . c-abi)
    (repr . gerbil-value-handle)
    (ownership . gerbil-runtime-owned)
    (nullability . non-zero-handle)
    (predicate . u8vector?)
    (projection . SchemeBytevector)
    (accessors (length u8-ref to-vec to-bytestring))
    (rooting . unrooted-borrow)
    (gc-policy . no-gc-root-guarantee)))

(def gerbil_scheme_rust_rooted_bytes_shape
  '(native-shape
    (name . rooted-bytes)
    (transport . c-abi)
    (repr . positive-i64-root-token)
    (conversions (u8vector->bytestring bytestring->u8vector))
    (aliases (bytevector->bytestring bytestring->bytevector))
    (safe-types (RootedSchemeString RootedSchemeBytevector))
    (delimiter . compact-or-unicode-scalar)
    (hex-case . uppercase)
    (rooting . scheme-module-root-table)
    (release . rust-raii-drop)
    (thread-affinity . runtime-owner-thread)
    (failure-policy . zero-root-to-invalid-value)))

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
    (scalar-values (i64 bool comparison status fixnum char flonum))
    (sentinel-values (nil void))
    (borrowed-values (bytevector utf8))
    (rooted-values (bytestring bytevector))
    (handle-values (runtime-handle gerbil-value-handle))
    (callback-values (i64-callback))
    (nullability . explicit-per-shape)
    (rooting . explicit-per-shape)))

(def gerbil_scheme_rust_native_error_shape
  '(native-shape
    (name . native-error)
    (transport . rust-safe-boundary)
    (taxonomy
     (already-initialized . gerbil-status)
     (runtime-finalized . gerbil-status)
     (invalid-lifecycle-state . rust-internal)
     (status . gerbil-status-code-preserving)
     (abi-mismatch . gerbil-status)
     (wrong-thread . gerbil-status)
     (integer-overflow . gerbil-status)
     (invalid-comparison-result . gerbil-status))
    (unknown-status-policy . preserve-code)
    (projection . optional-gerbil-status)
    (display-policy . operation-context-preserving)))

(def gerbil_scheme_rust_native_result_shape
  '(native-shape
    (name . native-result)
    (ok . native-value)
    (error . native-error)
    (status-projection . optional-gerbil-status)
    (unknown-status-policy . preserve-code)
    (failure-policy . fail-closed)))
