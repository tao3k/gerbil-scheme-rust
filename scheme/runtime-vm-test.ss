;;; SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
;;; Separate from runtime-smp-test: two VMs, one fixed OS owner each.
(import :gerbil-scheme-rust/scheme/runtime)
(export main)
(extern namespace: #f
  gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref
  gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release!)
(extern vm-overlap vm-role vm-run)

(def retained-root #f)
(def owner-marker #f)

(def (exercise owner)
  (set! owner-marker owner)
  (set! retained-root (gerbil-rs-root-object (vector owner (make-string 8192 #\a))))
  (##gc)
  (unless (= (vm-overlap) 1)
    (error "two VM owners did not overlap in Scheme execution"))
  (unless (and (= owner-marker owner) (= (##current-processor-id) 0)
               (= (##current-vm-processor-count) 1)
               (= (vector-ref
                   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref retained-root) 0)
                  owner))
    (error "VM globals/root/processor isolation failed"))
  owner)

(def (retained owner)
  (##gc)
  (unless (and (= owner-marker owner)
               (= (vector-ref
                   (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-ref retained-root) 0)
                  owner))
    (error "other VM root changed after independent VM destruction"))
  (unless (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-value-release! retained-root)
    (error "VM root release failed"))
  owner)

(begin-foreign
  (namespace ("gerbil-scheme-rust/scheme/runtime-vm-test#"
              vm-overlap vm-role vm-run exercise-entry retained-entry))
  (c-declare "#include \"runtime-vm-test-support.h\"")
  (define vm-overlap (c-lambda () int "___result = gerbil_vm_overlap();"))
  (define vm-role (c-lambda () int "___result = gerbil_vm_role;"))
  (define vm-run (c-lambda () int "___result = gerbil_vm_test_run();"))
  (c-define (exercise-entry owner) (int) int "gerbil_rust_vm_test_exercise" "extern"
    (gerbil-scheme-rust/scheme/runtime-vm-test#exercise owner))
  (c-define (retained-entry owner) (int) int "gerbil_rust_vm_test_retained" "extern"
    (gerbil-scheme-rust/scheme/runtime-vm-test#retained owner)))

(def (main . _)
  ;; The compiler-owned program is initialized in every VM. Secondary owners
  ;; finish initialization without recursively launching another owner pair.
  (when (= (vm-role) 0)
    (let (outcome (vm-run))
      (unless (= outcome 0)
        (error "shared-runtime independent VM qualification failed" outcome))
      (displayln "MULTI-VM two-owners processor0 overlap GC-roots independent-cleanup=OK"))))
