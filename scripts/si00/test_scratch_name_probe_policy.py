"""Required native scratch-name witnesses; synthetic receipts are not qualification."""
import ast
import json
import re
import unittest
from pathlib import Path
import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "store::foundation::scratch_name_probe::tests::"
COMMON = ['name_probe_precancel_precedes_scratch_allocation', 'name_probe_expired_deadline_precedes_scratch_allocation', 'name_probe_whole_plan_budgets_precede_allocation', 'name_probe_empty_tree_platform_contract_is_explicit']
UNIX = ['name_probe_whole_tree_preserves_exact_names_and_implicit_parents', 'name_probe_sibling_markers_coexist_until_all_names_are_tested', 'name_probe_case_and_unicode_results_follow_native_creation_not_folding', 'name_probe_same_leaf_name_in_different_parents_is_not_a_collision', 'name_probe_late_native_creation_failure_cleans_prior_allocations', 'name_probe_post_creation_cancellation_is_cleaned_and_not_success', 'name_probe_terminal_cancellation_after_cleanup_is_not_success', 'name_probe_primary_and_cleanup_failures_are_both_preserved', 'name_probe_cleanup_failure_alone_prevents_an_observation', 'name_probe_replaced_marker_and_unknown_bytes_are_not_deleted']
REQUIRED = [PREFIX + n for n in COMMON] + [PREFIX + "unix::" + n for n in UNIX]

class ScratchNameProbePolicyTests(unittest.TestCase):
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

    def test_exact_names_and_platform_boundaries(self):
        policy = json.loads((ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text())
        base = ROOT / "crates/lightr-store/src/store/foundation"
        for filename, expected in (("scratch_name_probe_tests.rs", COMMON), ("scratch_name_probe_unix_tests.rs", UNIX)):
            self.assertEqual(re.findall(r"^fn (name_probe_\w+)\(", (base / filename).read_text(), re.M), expected)
        self.assertEqual([n for n in policy["required_tests"]["lightr_store"] if n.startswith(PREFIX)], REQUIRED[:len(COMMON)])
        for name, profile in policy["profiles"].items():
            actual = [n for n in profile.get("required_tests", {}).get("lightr_store", []) if n.startswith(PREFIX)]
            self.assertEqual(actual, REQUIRED[len(COMMON):] if name.startswith(("linux-", "macos-")) else [])

    def test_complete_synthetic_receipt_is_not_qualification(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], len(REQUIRED) + 1)
        self.assertFalse(result["runtime_qualified"])

    def test_every_missing_method_is_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(missing=name).validate()

    def test_every_ignored_method_is_rejected_even_if_allowlisted(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(ignored=name).validate()

    def test_every_failed_method_is_rejected_despite_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(failed=name).validate()

    def test_old_and_new_control_families_are_both_required(self):
        path = ROOT / "scripts/si01/tree_plan_controls.py"
        text = path.read_text()
        tree = ast.parse(text)
        values = {n.targets[0].id: ast.literal_eval(n.value) for n in tree.body
                  if isinstance(n, ast.Assign) and isinstance(n.targets[0], ast.Name)
                  and n.targets[0].id in ("CASES", "NAME_CASES")}
        self.assertEqual(len(values["CASES"]), 6)
        self.assertEqual(len(values["NAME_CASES"]), 3)
        self.assertIn("in CASES + NAME_CASES:", text)
        for label, filename, old, _new, test, _message in values["NAME_CASES"]:
            source = ROOT / "crates/lightr-store/src/store/foundation" / filename
            self.assertEqual(source.read_text().count(old), 1, label)
            self.assertIn(test, REQUIRED)
        for stage in ("pristine", "restored"):
            self.assertEqual(text.count(f'run(root, name_suite, "names-{stage}")'), 1)
        self.assertIn('name_expected = r"test result: ok\\. 14 passed; 0 failed; 0 ignored;"', text)

if __name__ == "__main__":
    unittest.main()
