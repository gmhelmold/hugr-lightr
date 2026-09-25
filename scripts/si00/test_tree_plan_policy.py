"""Required logical-tree witnesses: synthetic receipts, not filesystem qualification."""
import json
import re
import unittest
from pathlib import Path
import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "store::foundation::tree_plan::tests::"
NAMES = ['tree_plan_empty_tree_fits_zero_budgets',
 'tree_plan_unsorted_tree_preserves_entries_and_includes_implied_directories',
 'tree_plan_duplicate_entries_of_every_kind_are_rejected',
 'tree_plan_non_directory_ancestors_are_rejected_in_all_entry_orders',
 'tree_plan_shared_implied_parents_and_sibling_prefixes_are_distinct',
 'tree_plan_names_are_exact_relative_components_not_normalized_paths',
 'tree_plan_preserves_unicode_case_and_unix_literal_characters',
 'tree_plan_link_text_is_retained_without_lookup_or_normalization',
 'tree_plan_wire_lengths_count_utf8_bytes_without_claiming_native_support',
 'tree_plan_declared_totals_and_sum_overflow_are_rejected',
 'tree_plan_version_and_permission_bit_contracts_are_explicit',
 'tree_plan_caller_budgets_include_repeated_components_and_link_text',
 'tree_plan_entry_cancellation_and_deadline_precede_validation',
 'tree_plan_every_checkpoint_failure_is_terminal_without_input_mutation',
 'tree_plan_small_tree_results_match_independent_pairwise_oracle']
REQUIRED = [PREFIX + name for name in NAMES]

class TreePlanPolicyTests(unittest.TestCase):
    def fixture(self, missing=None, ignored=None, failed=None):
        sample = fixtures.NativeEvidenceTests()
        sample.setUp()
        self.addCleanup(sample.doCleanups)
        sample.policy["required_tests"]["store"] = REQUIRED[:]
        statuses = {n: "ignored" if n == ignored else "FAILED" if n == failed else "ok"
                    for n in REQUIRED if n != missing}
        (sample.root / "list-store.stdout").write_text("".join(n + ": test\n" for n in statuses))
        counts = [sum(v == status for v in statuses.values()) for status in ("ok", "FAILED", "ignored")]
        verdict = "FAILED" if failed else "ok"
        (sample.root / "run-store.stdout").write_text(
            "".join(f"test {n} ... {v}\n" for n, v in statuses.items()) +
            f"test result: {verdict}. {counts[0]} passed; {counts[1]} failed; {counts[2]} ignored; "
            "0 measured; 0 filtered out\n")
        if ignored:
            sample.policy["allowed_ignored"][ignored] = "Synthetic rejection only"
        sample.rehash()
        return sample

    def test_registration_requires_all_methods_on_every_native_profile(self):
        policy = json.loads((ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text())
        source = ROOT / "crates/lightr-store/src/store/foundation/tree_plan_tests.rs"
        self.assertEqual(re.findall(r"^fn (tree_plan_[^(]+)\(", source.read_text(), re.M), NAMES)
        self.assertEqual([n for n in policy["required_tests"]["lightr_store"] if n.startswith(PREFIX)], REQUIRED)
        for profile in policy["profiles"].values():
            self.assertFalse(any(n.startswith(PREFIX) for n in profile.get("required_tests", {}).get("lightr_store", [])))

    def test_complete_synthetic_receipt_is_baseline_only(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], 16)
        self.assertFalse(result["runtime_qualified"])

    def test_every_missing_tree_witness_is_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(missing=name).validate()

    def test_every_ignored_tree_witness_is_rejected_even_if_allowlisted(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(ignored=name).validate()

    def test_every_failed_tree_witness_is_rejected_despite_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(failed=name).validate()

if __name__ == "__main__":
    unittest.main()
