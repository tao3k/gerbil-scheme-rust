"""Validate optional diagnostics on the real pinned native API, not a mock VM."""
import json
import unittest

import gerbil_ledger


class OwnerProfilingTest(unittest.TestCase):
    def test_native_phase_counters_and_host_boundaries(self):
        library = gerbil_ledger.open_library()
        try:
            session = library.session(0)
            library.set_owner_profiling(True)
            roots = session.call_many([("echo", 1), ("gc",), ("echo", 2)])
            rows = library.project_many(roots, release=True)
            self.assertEqual((rows[0], rows[-1]), (1, 2))
            trace = json.loads(library.owner_profile())
            self.assertEqual(trace["dropped"], 0)
            spans = trace["spans"]
            kinds = {span["kind"] for span in spans}
            self.assertTrue({"python-session-read-many", "gil-return-session",
                             "python-session-wrap-results", "python-value-batch-read",
                             "python-materialize-many", "rust-projection-wait",
                             "rust-release-wait", "gil-return-projection",
                             "gil-return-release"} <= kinds)
            calls = [span for span in spans if span["kind"] == "batch-call"]
            self.assertEqual(len(calls), 3)
            self.assertTrue(all(span["owner"] == 0 for span in calls))
            self.assertTrue(all(0 <= span["stages_ns"][0] <= span["stages_ns"][1]
                                <= span["end_ns"] - span["start_ns"] for span in calls))
            self.assertGreaterEqual(sum(span["stage_diagnostics_delta"][1][5]
                                        for span in calls), 1)
            with self.assertRaises(RuntimeError):
                session.call_many([("append", "not-an-integer")])
            failure = json.loads(library.owner_profile())
            failed = [span for span in failure["spans"] if span["kind"] == "batch-call"]
            self.assertEqual(len(failed), 1)
            self.assertTrue(failed[0]["failed"])
            self.assertIsNone(failed[0]["stage_diagnostics_delta"])
            roots = session.call_many([("echo", 42)])
            self.assertEqual(library.project_many(roots, release=True), [42])
            library.owner_profile()
            library.set_owner_profiling(False)
            root = session.call(("echo", 42))
            self.assertEqual(root.to_python(), 42)
            root.close()
            self.assertEqual(json.loads(library.owner_profile())["spans"], [])
        finally:
            library.close()
        self.assertTrue(library.shutdown_clean)
        self.assertEqual(library.shutdown_roots, [0] * 4)


if __name__ == "__main__":
    unittest.main()
