"""Mandatory managed-scratch witnesses; synthetic policy checks are not native tests."""
import json
import re
import unittest
from pathlib import Path
import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "store::foundation::anchored_scratch::"
COMMON = ['scratch_precancel_precedes_allocation', 'scratch_expired_deadline_precedes_allocation', 'scratch_platform_contract_is_explicit']
UNIX = ['scratch_round_trip_and_checked_cleanup', 'scratch_existing_names_are_never_adopted_or_truncated', 'scratch_invalid_components_do_not_access_other_paths', 'scratch_anchor_ignores_replaced_path_spelling', 'scratch_cleanup_preserves_replacement_entries', 'scratch_unknown_children_prevent_recursive_cleanup', 'scratch_drop_removes_only_owned_names', 'scratch_child_cancellation_does_not_allocate', 'scratch_allocations_progress_independently', 'scratch_modes_and_lease_exclusion_remain_private']
NATIVE = ['scratch_atomic_reservation_has_a_finite_collision_budget', 'scratch_owned_descriptors_are_close_on_exec']
REQUIRED = [PREFIX + "tests::" + n for n in COMMON] + [PREFIX + "tests::unix::" + n for n in UNIX] + [PREFIX + "native::tests::" + n for n in NATIVE]


class AnchoredScratchPolicyTests(unittest.TestCase):
    def fixture(self, missing=None, ignored=None, failed=None):
        sample = fixtures.NativeEvidenceTests()
        sample.setUp()
        self.addCleanup(sample.doCleanups)
        sample.policy["required_tests"]["store"] = []
        sample.policy["profiles"]["native"]["required_tests"] = {"store": REQUIRED[:]}
        statuses = {name: "ignored" if name == ignored else "FAILED" if name == failed else "ok"
                    for name in REQUIRED if name != missing}
        (sample.root / "list-store.stdout").write_text("".join(name + ": test\n" for name in statuses))
        counts = [sum(v == status for v in statuses.values()) for status in ("ok", "FAILED", "ignored")]
        verdict = "FAILED" if failed else "ok"
        (sample.root / "run-store.stdout").write_text(
            "".join(f"test {name} ... {status}\n" for name, status in statuses.items()) +
            f"test result: {verdict}. {counts[0]} passed; {counts[1]} failed; {counts[2]} ignored; "
            "0 measured; 0 filtered out\n")
        if ignored:
            sample.policy["allowed_ignored"][ignored] = "Synthetic rejection only"
        sample.rehash()
        return sample

    def test_exact_names_and_platform_boundaries_are_registered(self):
        policy = json.loads((ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text())
        base = ROOT / "crates/lightr-store/src/store/foundation"
        for file, expected in [("anchored_scratch_tests.rs", COMMON), ("anchored_scratch_unix_tests.rs", UNIX), ("anchored_scratch_native.rs", NATIVE)]:
            names = re.findall(r"^\s*fn (scratch_\w+)\(", (base / file).read_text(), re.M)
            self.assertEqual(names, expected)
        shared = [n for n in policy["required_tests"]["lightr_store"] if n.startswith(PREFIX)]
        self.assertEqual(shared, REQUIRED[:len(COMMON)])
        for profile, config in policy["profiles"].items():
            actual = [n for n in config.get("required_tests", {}).get("lightr_store", []) if n.startswith(PREFIX)]
            expected = REQUIRED[len(COMMON):] if profile.startswith(("linux-", "macos-")) else []
            self.assertEqual(actual, expected)

    def test_new_causal_seams_are_unique_and_old_families_are_preserved(self):
        import sys
        sys.path.insert(0, str(ROOT / "scripts/si01"))
        try:
            import topology_controls as controls
            self.assertEqual(len(controls.CASES), 10)
            self.assertEqual(len(controls.EMPTY_CASES), 2)
            self.assertEqual(len(controls.SCRATCH_CASES), 3)
            for label, filename, old, _new, test, _message in controls.SCRATCH_CASES:
                text = (ROOT / controls.BASE / filename).read_text()
                self.assertEqual(text.count(old), 1, label)
                self.assertIn(test, REQUIRED)
        finally:
            sys.path.pop(0)

    def test_composition_keeps_descendant_and_scratch_controls_and_suites(self):
        import ast
        text = (ROOT / "scripts/si01/topology_controls.py").read_text()
        tree = ast.parse(text)
        controls = [n for n in ast.walk(tree) if isinstance(n, ast.For)
                    and isinstance(n.target, ast.Tuple)
                    and any(isinstance(item, ast.Name) and item.id == "label"
                            for item in n.target.elts)]
        self.assertEqual(len(controls), 1)
        names = [n.id for n in ast.walk(controls[0].iter) if isinstance(n, ast.Name)]
        self.assertCountEqual(names, ["CASES", "EMPTY_CASES", "DESCENDANT_CASES", "SCRATCH_CASES", "LINK_CASES", "LISTING_CASES"])
        for family, count in (("descendant", 13), ("scratch", 15)):
            self.assertIn(f'{family}_expected = r"test result: ok\\. {count} passed;', text)
            for stage in ("pristine", "restored"):
                self.assertEqual(text.count(f'run(root, {family}_suite, "{family}-{stage}")'), 1)

    def test_complete_synthetic_receipt_is_not_runtime_qualification(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], len(REQUIRED) + 1)
        self.assertFalse(result["runtime_qualified"])

    def test_missing_scratch_methods_are_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(missing=name).validate()

    def test_ignored_scratch_methods_are_rejected_even_when_allowed(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(ignored=name).validate()

    def test_failed_scratch_methods_are_rejected_despite_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(failed=name).validate()


if __name__ == "__main__":
    unittest.main()
