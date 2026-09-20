"""Required destination-anchor witnesses; synthetic receipts are not native qualification."""
import ast
import json
import re
import sys
import unittest
from pathlib import Path

import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "store::foundation::destination_anchor::tests::"
NATIVE = [
    "anchor_existing_empty_uses_exact_directory_and_rollback_never_removes_it",
    "anchor_existing_nonempty_is_rejected_without_touching_user_data",
    "anchor_missing_nested_destination_creates_exact_owned_suffix_and_rolls_back",
    "anchor_missing_alias_parent_uses_native_resolution_not_lexical_parent",
    "anchor_never_adopts_a_name_that_appears_after_preflight",
    "anchor_cancellation_after_first_creation_rolls_back_owned_prefix",
    "anchor_final_validation_rejects_replacement_and_preserves_decoy",
    "anchor_existing_replacement_after_empty_scan_is_rejected_without_cleanup",
    "anchor_rollback_preserves_unknown_output_and_reports_partial_state",
    "anchor_created_handle_is_close_on_exec_and_revalidates",
    "anchor_revalidates_planned_protected_roots_before_mutation",
]
REQUIRED = [PREFIX + name for name in NATIVE]
FAMILIES = [
    "CASES", "EMPTY_CASES", "DESCENDANT_CASES", "SCRATCH_CASES",
    "LINK_CASES", "LISTING_CASES", "DEST_ANCHOR_CASES",
]

class DestinationAnchorPolicyTests(unittest.TestCase):
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
        source = (
            ROOT / "crates/lightr-store/src/store/foundation/destination_anchor_tests.rs"
        ).read_text()
        declared = re.findall(r"^fn (anchor_\w+)\(", source, re.M)
        self.assertEqual(declared, NATIVE)
        self.assertFalse(
            any(name.startswith(PREFIX) for name in policy["required_tests"]["lightr_store"])
        )
        for profile, record in policy["profiles"].items():
            required = record.get("required_tests", {}).get("lightr_store", [])
            expected = REQUIRED if profile.startswith(("linux-", "macos-")) else []
            self.assertEqual([n for n in required if n.startswith(PREFIX)], expected)
            self.assertEqual(len(required), len(set(required)))

    def test_control_family_is_wired_without_dropping_prior_controls(self):
        sys.path.insert(0, str(ROOT / "scripts/si01"))
        try:
            import topology_controls as controls
            self.assertEqual(len(controls.DEST_ANCHOR_CASES), 3)
            text = (ROOT / "scripts/si01/topology_controls.py").read_text()
            loops = [
                node for node in ast.walk(ast.parse(text))
                if isinstance(node, ast.For)
                and isinstance(node.target, ast.Tuple)
                and any(isinstance(v, ast.Name) and v.id == "label" for v in node.target.elts)
            ]
            self.assertEqual(len(loops), 1)
            self.assertCountEqual(
                [n.id for n in ast.walk(loops[0].iter) if isinstance(n, ast.Name)],
                FAMILIES,
            )
            for label, filename, old, _new, test, _message in controls.DEST_ANCHOR_CASES:
                self.assertEqual(
                    (ROOT / controls.BASE / filename).read_text().count(old), 1, label
                )
                self.assertIn(test, REQUIRED)
            self.assertIn(
                'anchor_expected = r"test result: ok\\. 11 passed; 0 failed; 0 ignored;"',
                text,
            )
            for stage in ("pristine", "restored"):
                self.assertEqual(
                    text.count(f'run(root, anchor_suite, "anchor-{stage}")'), 1
                )
        finally:
            sys.path.pop(0)

    def test_complete_synthetic_receipt_is_not_runtime_qualification(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], len(REQUIRED) + 1)
        self.assertFalse(result["runtime_qualified"])

    def test_every_missing_anchor_witness_is_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(missing=name).validate()

    def test_every_ignored_anchor_witness_is_rejected_even_if_allowlisted(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(ignored=name).validate()

    def test_every_failed_anchor_witness_is_rejected_despite_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(failed=name).validate()

if __name__ == "__main__":
    unittest.main()
