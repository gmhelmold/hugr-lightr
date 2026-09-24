"""Cancellation harness selection regression: no compiler or filesystem fixture."""
import runpy
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONTROL = runpy.run_path(str(ROOT / "scripts/si01/cancellation_controls.py"))

class CancellationSelectionTests(unittest.TestCase):
    def test_original_namespaces_counts_and_artifact_labels_remain_exact(self):
        self.assertEqual(CONTROL["SUITES"], [
            ("lightr-core", "core::digest::checked_tests::checked_hash_", "checked_hash_", 7),
            ("lightr-store", "store::foundation::capture_cancellation_tests::capture_cancellation_", "capture_cancellation_", 8),
            ("lightr-store", "store::cas::preparation::cancellation_tests::checkpoint_", "checkpoint_", 7),
        ])

    def test_checkpoint_family_does_not_absorb_unrelated_tree_test(self):
        _, selection, _, _ = CONTROL["SUITES"][2]
        original = ["checkpoint_hash_cancellation_cleans_only_owned_stage",
                    "checkpoint_precancel_allocates_nothing",
                    "checkpoint_short_write_cancel_stops_remaining_bytes",
                    "checkpoint_short_writes_and_os_interrupts_still_complete",
                    "checkpoint_sync_completion_cannot_hide_cancellation",
                    "checkpoint_write_interruption_is_not_blindly_retried",
                    "checkpoint_writer_errors_and_invalid_counts_remain_explicit"]
        names = ["store::cas::preparation::cancellation_tests::" + n for n in original]
        unrelated = "store::foundation::tree_plan::tests::tree_plan_every_checkpoint_failure_is_terminal_without_input_mutation"
        self.assertEqual([n for n in names + [unrelated] if selection in n], names)
        self.assertIn("checkpoint_", unrelated)  # The old broad selector included it.

if __name__ == "__main__":
    unittest.main()
