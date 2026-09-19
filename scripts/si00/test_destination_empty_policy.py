"""Mandatory native emptiness names; synthetic receipts are not Rust execution."""
import ast
import json
import re
import unittest
from pathlib import Path

import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = 'store::foundation::topology::topology_empty::tests::'
COMMON = ['empty_destination_existing_and_missing_are_checked_without_creation', 'empty_destination_counts_hidden_files_directories_and_dangling_links', 'empty_destination_scan_has_an_independent_directory_offset', 'empty_destination_rechecks_contents_without_freezing_them', 'empty_destination_rejects_replacement_and_missing_target_adoption', 'empty_destination_alias_parent_keeps_native_not_lexical_target', 'empty_destination_cancellation_and_deadline_do_not_modify_output', 'empty_destination_scan_checks_cancellation_after_eof', 'empty_destination_scan_preserves_read_errors_and_bounds_dot_streams']
LINUX = ['empty_destination_non_utf8_name_is_an_entry_not_an_omission']
REQUIRED = [PREFIX + name for name in COMMON + LINUX]


class DestinationEmptyPolicyTests(unittest.TestCase):
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
        source = ROOT / "crates/lightr-store/src/store/foundation/topology_empty_tests.rs"
        declared = re.findall(r"^fn (empty_destination_[^(]+)\(", source.read_text(), re.M)
        self.assertEqual(declared, COMMON + LINUX)
        self.assertFalse(any(n.startswith(PREFIX) for n in policy["required_tests"]["lightr_store"]))
        for name, profile in policy["profiles"].items():
            required = profile.get("required_tests", {}).get("lightr_store", [])
            expected = COMMON + LINUX if name.startswith("linux-") else COMMON if name.startswith("macos-") else []
            self.assertEqual([n for n in required if n.startswith(PREFIX)], [PREFIX + n for n in expected])
            self.assertEqual(len(required), len(set(required)))

    def test_causal_selectors_keep_old_and_new_families_separate(self):
        source = (ROOT / "scripts/si01/topology_controls.py").read_text()
        tree = ast.parse(source)
        values = {}
        for node in ast.walk(tree):
            if isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name) and target.id in {"suite", "empty_suite"}:
                        values[target.id] = ast.unparse(node.value)
        self.assertEqual(values["suite"], "cargo + [PREFIX + 'tests::', '--', '--nocapture']")
        self.assertEqual(values["empty_suite"], "cargo + [PREFIX + 'topology_empty::tests::', '--', '--nocapture']")
        self.assertIn('"empty-independent-offset"', source)
        self.assertIn('"empty-post-eof-cancellation"', source)

    def test_complete_synthetic_receipt_is_not_runtime_qualification(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], len(REQUIRED) + 1)
        self.assertFalse(result["runtime_qualified"])

    def test_missing_empty_destination_methods_are_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(missing=name).validate()

    def test_ignored_empty_destination_methods_are_rejected_even_when_allowed(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(ignored=name).validate()

    def test_failed_empty_destination_methods_are_rejected_despite_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(failed=name).validate()


if __name__ == "__main__":
    unittest.main()
