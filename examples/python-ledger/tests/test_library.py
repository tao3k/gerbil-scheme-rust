"""Installed-wheel qualification; never imports source or invokes a compiler."""
import concurrent.futures
import os
import unittest

import gerbil_ledger


class InstalledLibraryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from gerbil_ledger._core import PythonLibrary

        cls.before_initialization = PythonLibrary.capabilities()
        cls.library = gerbil_ledger.open_library(backend="scheme")

    @classmethod
    def tearDownClass(cls):
        value = cls.library.call(("echo", "live before close"))
        cls.library.close()
        cls.library.close()
        try:
            value.to_python()
        except RuntimeError:
            pass
        else:
            raise AssertionError("closed library retained a callable native root")
        value.close()

    def echo(self, value):
        root = self.library.call(("echo", value))
        try:
            return root.to_python()
        finally:
            root.close()

    def test_exact_scalars_and_binary_data(self):
        for value in [None, False, True, 0, -(2**137), 2**149, 1.25, "λ Scheme", b"\x00\xff"]:
            with self.subTest(value=value):
                self.assertEqual(self.echo(value), value)

    def test_complete_package_reverification(self):
        self.library.verify_resources()

    def test_linked_vm_storage_is_not_execution_admission(self):
        capabilities = self.library.capabilities()
        self.assertEqual(self.library.active_processors, 1)
        self.assertEqual(capabilities.execution_model, "process-global-single-vm-owner")
        self.assertFalse(capabilities.independent_vm_execution)
        self.assertIsInstance(capabilities.multiple_vm_storage, bool)
        self.assertIsInstance(capabilities.thread_local_entry, bool)
        self.assertGreaterEqual(capabilities.max_processors, 1)
        self.assertEqual(
            capabilities.multiple_vm_storage,
            self.before_initialization.multiple_vm_storage,
        )
        self.assertEqual(
            capabilities.thread_local_entry,
            self.before_initialization.thread_local_entry,
        )

    def test_second_vm_is_not_silently_aliased_to_first(self):
        with self.assertRaises(RuntimeError):
            gerbil_ledger.open_library(backend="scheme")
        self.assertEqual(self.echo("first owner survives rejected second VM"),
                         "first owner survives rejected second VM")

    def test_runtime_loaded_hygienic_dsl_component(self):
        result = self.library.call(("dsl", "(let ((foldl #f)) (ledger-sum 20 22))"))
        self.assertEqual(result.to_python(), 42)
        result.close()

    def test_native_shared_identity_survives_parent_release_and_gc(self):
        parent = self.library.call(("shared",))
        first, second = parent.elements()
        self.assertTrue(first.is_identical(second))
        parent.close()
        self.library.call(("gc",)).close()
        self.assertEqual(first.elements()[0].to_python(), "retained")
        self.assertTrue(first.is_identical(second))
        first.close()
        second.close()

    def test_cyclic_native_object_is_retained_not_json_projected(self):
        cycle = self.library.call(("cycle",))
        child = cycle.elements()[0]
        self.assertTrue(cycle.is_identical(child))
        with self.assertRaises(RuntimeError):
            cycle.to_python()
        self.library.call(("gc",)).close()
        self.assertTrue(cycle.is_identical(child))
        cycle.close()
        self.assertEqual(child.kind, "Vector")
        child.close()

    def test_closure_execution_does_not_require_scalar_projection(self):
        procedure = self.library.call(("procedure",))
        self.assertEqual(procedure.kind, "Procedure")
        with self.assertRaises(RuntimeError):
            procedure.to_python()
        self.assertEqual(procedure.apply(2**130).to_python(), 2**130 + 1)
        procedure.close()

    def test_python_collections_remain_scheme_collections(self):
        root = self.library.call(("echo", [2, 3, 5]))
        self.assertEqual([v.to_python() for v in root.elements()], [2, 3, 5])
        root.close()
        table = self.library.call(("echo", {"a": 1, "b": 2}))
        self.assertEqual(table.kind, "GambitTable")
        flat = [v.to_python() for v in table.elements()]
        self.assertEqual(dict(zip(flat[::2], flat[1::2])), {"a": 1, "b": 2})
        table.close()

    def test_bad_export_call_does_not_destroy_input_or_owner(self):
        request = self.library.value(("unknown",))
        with self.assertRaises(RuntimeError):
            self.library.call(request)
        self.assertEqual(request.kind, "Vector")
        self.assertEqual(self.echo("recovered"), "recovered")
        request.close()

    def test_released_reference_is_rejected(self):
        request = self.library.value("released")
        request.close()
        request.close()
        with self.assertRaises(RuntimeError):
            self.library.call(("echo", request))

    def test_python_cycles_are_explicitly_rejected(self):
        value = []
        value.append(value)
        with self.assertRaises(ValueError):
            self.library.value(value)
        self.assertEqual(self.echo(42), 42)

    def test_concurrent_python_hosts_do_not_cross_vm_affinity(self):
        def submit(value):
            return self.echo(value)

        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
            self.assertEqual(list(pool.map(submit, range(32))), list(range(32)))

    def test_forked_host_is_rejected_before_inherited_locks(self):
        if not hasattr(os, "fork"):
            self.skipTest("fork is unsupported on this platform; not an ownership PASS")
        pid = os.fork()
        if pid == 0:
            try:
                self.library.value(1)
            except RuntimeError:
                os._exit(0)
            os._exit(1)
        _, status = os.waitpid(pid, 0)
        self.assertEqual(os.waitstatus_to_exitcode(status), 0)
        self.assertEqual(self.echo(7), 7)


if __name__ == "__main__":
    unittest.main()
