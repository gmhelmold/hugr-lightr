"""Mandatory actual-destination name witnesses; synthetic receipts are not native qualification."""
import ast
import json
import re
import sys
import unittest
from pathlib import Path

import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "store::foundation::destination_name_probe::tests::"
NATIVE = [
    'destination_name_probe_empty_tree_is_noop',
    'destination_name_probe_whole_tree_uses_actual_root_and_cleans',
    'destination_name_probe_link_entries_are_name_markers_not_links',
    'destination_name_probe_native_case_and_unicode_match_same_directory_oracle',
    'destination_name_probe_budgets_precede_mutation',
    'destination_name_probe_late_native_error_cleans_prior_names',
    'destination_name_probe_created_handles_are_close_on_exec',
    'destination_name_probe_created_anchor_root_can_rollback_after_success',
    'destination_name_probe_never_adopts_foreign_name_after_empty_check',
    'destination_name_probe_cancellation_cleans_created_names',
    'destination_name_probe_replacement_is_preserved_and_reports_incomplete_cleanup',
    'destination_name_probe_unknown_child_blocks_recursive_cleanup',
    'destination_name_probe_final_empty_check_detects_foreign_root_entry',
    'destination_name_probe_final_binding_rejects_root_replacement',
]
CORE_COUNT = 8
REQUIRED = (
    [PREFIX + name for name in NATIVE[:CORE_COUNT]]
    + [PREFIX + "races::" + name for name in NATIVE[CORE_COUNT:]]
)
FAMILIES = [
    "CASES", "EMPTY_CASES", "DESCENDANT_CASES", "SCRATCH_CASES",
    "LINK_CASES", "LISTING_CASES", "DEST_ANCHOR_CASES", "DEST_REPR_CASES", "DEST_PREPARE_CASES",
]


class DestinationNameProbePolicyTests(unittest.TestCase):
    def fixture(self, missing=None, ignored=None, failed=None):
        sample = fixtures.NativeEvidenceTests()
        sample.setUp()
        self.addCleanup(sample.doCleanups)
        sample.policy["required_tests"]["store"] = []
        sample.policy["profiles"]["native"]["required_tests"] = {"store": REQUIRED[:]}
        statuses = {
            name: "ignored" if name == ignored else "FAILED" if name == failed else "ok"
            for name in REQUIRED if name != missing
        }
        (sample.root / "list-store.stdout").write_text(
            "".join(name + ": test\n" for name in statuses)
        )
        counts = [
            sum(value == status for value in statuses.values())
            for status in ("ok", "FAILED", "ignored")
        ]
        verdict = "FAILED" if failed else "ok"
        (sample.root / "run-store.stdout").write_text(
            "".join(f"test {name} ... {status}\n" for name, status in statuses.items())
            + f"test result: {verdict}. {counts[0]} passed; {counts[1]} failed; "
              f"{counts[2]} ignored; 0 measured; 0 filtered out\n"
        )
        if ignored:
            sample.policy["allowed_ignored"][ignored] = "Synthetic rejection only"
        sample.rehash()
        return sample

    def test_exact_names_and_platform_boundaries_are_registered(self):
        policy = json.loads(
            (ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text()
        )
        base = ROOT / "crates/lightr-store/src/store/foundation"
        declared = []
        for filename in (
            "destination_name_probe_tests.rs",
            "destination_name_probe_race_tests.rs",
        ):
            declared.extend(
                re.findall(
                    r"^fn (destination_name_probe_\w+)\(",
                    (base / filename).read_text(),
                    re.M,
                )
            )
        self.assertEqual(declared, NATIVE)
        self.assertFalse(
            any(name.startswith(PREFIX) for name in policy["required_tests"]["lightr_store"])
        )
        for profile, record in policy["profiles"].items():
            required = record.get("required_tests", {}).get("lightr_store", [])
            expected = REQUIRED if profile.startswith(("linux-", "macos-")) else []
            self.assertEqual([name for name in required if name.startswith(PREFIX)], expected)
            self.assertEqual(len(required), len(set(required)))

    def test_controls_are_wired_without_dropping_prior_families(self):
        sys.path.insert(0, str(ROOT / "scripts/si01"))
        try:
            import topology_controls as controls
            self.assertEqual(len(controls.DEST_REPR_CASES), 4)
            for family, count in zip(FAMILIES, [10, 2, 3, 3, 4, 4, 3, 4, 7]):
                self.assertEqual(len(getattr(controls, family)), count, family)
            text = (ROOT / "scripts/si01/topology_controls.py").read_text()
            loops = [
                node for node in ast.walk(ast.parse(text))
                if isinstance(node, ast.For)
                and isinstance(node.target, ast.Tuple)
                and any(isinstance(value, ast.Name) and value.id == "label"
                        for value in node.target.elts)
            ]
            self.assertEqual(len(loops), 1)
            self.assertCountEqual(
                [node.id for node in ast.walk(loops[0].iter) if isinstance(node, ast.Name)],
                FAMILIES,
            )
            for label, filename, old, _new, test, _message in controls.DEST_REPR_CASES:
                self.assertEqual(
                    (ROOT / controls.BASE / filename).read_text().count(old), 1, label
                )
                self.assertIn(test, REQUIRED)
            self.assertIn(
                'repr_expected = r"test result: ok\\. 14 passed; 0 failed; 0 ignored;"',
                text,
            )
            for stage in ("pristine", "restored"):
                self.assertEqual(text.count(f'run(root, repr_suite, "repr-{stage}")'), 1)
        finally:
            sys.path.pop(0)

    def test_complete_synthetic_receipt_is_not_runtime_qualification(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], len(REQUIRED) + 1)
        self.assertFalse(result["runtime_qualified"])

    def test_every_missing_witness_is_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(missing=name).validate()

    def test_every_ignored_witness_is_rejected_even_if_allowlisted(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(ignored=name).validate()

    def test_every_failed_witness_is_rejected_despite_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(failed=name).validate()


if __name__ == "__main__":
    unittest.main()
