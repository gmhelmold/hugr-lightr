"""Mandatory prepared-destination witnesses; synthetic receipts are not native qualification."""
import ast
import json
import re
import sys
import unittest
from pathlib import Path

import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "store::foundation::destination_tree_prepare::tests::"
CORE = [
    "prepared_tree_empty_is_noop_and_rolls_back_cleanly",
    "prepared_tree_creates_exact_namespace_and_explicit_rollback_cleans_it",
    "prepared_file_handle_is_cloexec_and_ready_for_future_payload_writer",
    "prepared_entries_use_private_intermediate_modes",
    "prepared_tree_representation_probe_finishes_before_persistent_creation",
    "prepared_tree_late_native_failure_cleans_prior_entries",
    "prepared_tree_created_anchor_root_remains_for_anchor_rollback",
]
RACES = [
    "prepared_tree_never_adopts_foreign_file_after_representation",
    "prepared_tree_cancellation_after_first_entry_rolls_back_owned_names",
    "prepared_tree_replacement_is_preserved_and_cleanup_is_incomplete",
    "prepared_tree_unknown_child_prevents_recursive_directory_cleanup",
    "prepared_tree_final_binding_rejects_root_replacement",
    "prepared_tree_final_entry_revalidation_rejects_leaf_replacement",
    "prepared_tree_drop_best_effort_removes_uncommitted_entries",
    "prepared_tree_final_validation_rejects_unplanned_root_entry",
]
REQUIRED = [PREFIX + name for name in CORE] + [PREFIX + "races::" + name for name in RACES]
FAMILIES = [
    "CASES", "EMPTY_CASES", "DESCENDANT_CASES", "SCRATCH_CASES",
    "LINK_CASES", "LISTING_CASES", "DEST_ANCHOR_CASES", "DEST_REPR_CASES",
    "DEST_PREPARE_CASES",
]


class DestinationTreePreparePolicyTests(unittest.TestCase):
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
        expected = json.loads(
            (ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text()
        )
        base = ROOT / "crates/lightr-store/src/store/foundation"
        declared = []
        for filename in (
            "destination_tree_prepare_tests.rs",
            "destination_tree_prepare_race_tests.rs",
        ):
            declared.extend(
                re.findall(
                    r"^fn (prepared_\w+)\(",
                    (base / filename).read_text(),
                    re.M,
                )
            )
        self.assertEqual(declared, CORE + RACES)
        self.assertEqual(len(REQUIRED), 15)
        self.assertFalse(
            any(name.startswith(PREFIX) for name in expected["required_tests"]["lightr_store"])
        )
        for profile, record in expected["profiles"].items():
            required = record.get("required_tests", {}).get("lightr_store", [])
            platform_expected = REQUIRED if profile.startswith(("linux-", "macos-")) else []
            self.assertEqual(
                [name for name in required if name.startswith(PREFIX)],
                platform_expected,
            )
            self.assertEqual(len(required), len(set(required)))

    def test_controls_are_wired_without_dropping_prior_families(self):
        sys.path.insert(0, str(ROOT / "scripts/si01"))
        try:
            import topology_controls as controls
            self.assertEqual(len(controls.DEST_PREPARE_CASES), 5)
            for family, count in zip(FAMILIES, [10, 2, 3, 3, 4, 4, 3, 4, 5]):
                self.assertEqual(len(getattr(controls, family)), count, family)
            text = (ROOT / "scripts/si01/topology_controls.py").read_text()
            loops = [
                node for node in ast.walk(ast.parse(text))
                if isinstance(node, ast.For)
                and isinstance(node.target, ast.Tuple)
                and any(
                    isinstance(value, ast.Name) and value.id == "label"
                    for value in node.target.elts
                )
            ]
            self.assertEqual(len(loops), 1)
            self.assertCountEqual(
                [node.id for node in ast.walk(loops[0].iter) if isinstance(node, ast.Name)],
                FAMILIES,
            )
            for label, filename, old, _new, test, _message in controls.DEST_PREPARE_CASES:
                self.assertEqual(
                    (ROOT / controls.BASE / filename).read_text().count(old),
                    1,
                    label,
                )
                self.assertIn(test, REQUIRED)
            self.assertIn(
                'prepare_expected = r"test result: ok\\. 15 passed; 0 failed; 0 ignored;"',
                text,
            )
            for stage in ("pristine", "restored"):
                self.assertEqual(
                    text.count(f'run(root, prepare_suite, "prepare-{stage}")'),
                    1,
                )
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
