"""Benchmark protocol checks; these do not execute or qualify Scheme code."""
import unittest
from benchmark_metrics import assigned_owner_fairness, pending_completion_gaps


class CompletionFairnessTests(unittest.TestCase):
    def test_equal_coverage_can_hide_late_service(self):
        balanced = pending_completion_gaps([0, 1, 0, 1], 2)
        late = pending_completion_gaps([0, 0, 1, 1], 2)
        self.assertEqual(balanced["counts"], late["counts"])
        self.assertEqual(balanced["max_other_completions_while_pending"], [1, 1])
        self.assertEqual(late["max_other_completions_while_pending"], [0, 2])

    def test_completed_owner_has_no_artificial_trailing_wait(self):
        result = assigned_owner_fairness([1, 10, 2, 11], 2)
        self.assertEqual(result["assigned_owner_counts"], [2, 2])
        self.assertEqual(result["assigned_owner_max_pending_gap_ns"], [1, 10])
        self.assertEqual(result["assigned_owner_max_other_completions_while_pending"], [0, 2])

    def test_native_window_clock_restarts_and_owner_offset_continues(self):
        result = assigned_owner_fairness(list(range(1, 129)) + [2], 3)
        self.assertEqual(result["assigned_owner_counts"], [43, 43, 43])
        self.assertEqual(result["assigned_owner_max_pending_gap_ns"], [3, 3, 3])
        self.assertEqual(result["assigned_owner_max_other_completions_while_pending"], [2, 2, 2])

    def test_stateless_cursor_offset_is_preserved_across_native_windows(self):
        result = assigned_owner_fairness([1, 10, 2, 11], 3, first_owner=2)
        self.assertEqual(result["assigned_owner_counts"], [1, 1, 2])
        self.assertEqual(result["assigned_owner_max_pending_gap_ns"], [10, 2, 10])
        result = assigned_owner_fairness(list(range(1, 129)) + [2], 4, first_owner=3)
        self.assertEqual(result["assigned_owner_counts"], [32, 32, 32, 33])
        with self.assertRaises(ValueError): assigned_owner_fairness([1], 4, first_owner=4)

    def test_single_owner_and_empty_actor_groups(self):
        self.assertEqual(assigned_owner_fairness([3, 8], 1)["assigned_owner_max_pending_gap_ns"], [5])
        self.assertEqual(pending_completion_gaps([0, 0], 4)["counts"], [2, 0, 0, 0])


if __name__ == "__main__":
    unittest.main()
