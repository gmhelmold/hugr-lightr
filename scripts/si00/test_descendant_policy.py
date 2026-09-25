"""Mandatory native emptiness names; synthetic receipts are not Rust execution."""
import ast
import json
import re
import unittest
from pathlib import Path

import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = 'store::foundation::topology::topology_descendant::tests::'
COMMON = ['descendant_nested_file_and_directory_preserve_opened_identity', 'descendant_each_file_open_has_an_independent_offset', 'descendant_rejects_links_at_every_component', 'descendant_rejects_non_normal_relative_components', 'descendant_work_budgets_reject_before_traversal', 'descendant_native_missing_type_and_hardlink_errors_are_preserved', 'descendant_file_source_cannot_be_used_as_a_directory', 'descendant_revalidates_the_original_source_before_opening', 'descendant_revalidates_each_component_after_opening', 'descendant_opened_file_remains_the_original_after_later_rename', 'descendant_cancellation_deadline_and_scoped_errors_are_terminal', 'descendant_preserves_native_name_bytes_and_does_not_acquire_a_lease']
LINUX = ['descendant_linux_non_utf8_name_is_preserved']
REQUIRED = [PREFIX + name for name in COMMON + LINUX]


class DescendantPolicyTests(unittest.TestCase):
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
        source = ROOT / "crates/lightr-store/src/store/foundation/topology_descendant_tests.rs"
        declared = re.findall(r"^fn (descendant_[^(]+)\(", source.read_text(), re.M)
        self.assertEqual(declared, COMMON + LINUX)
        self.assertFalse(any(n.startswith(PREFIX) for n in policy["required_tests"]["lightr_store"]))
        for name, profile in policy["profiles"].items():
            required = profile.get("required_tests", {}).get("lightr_store", [])
            expected = COMMON + LINUX if name.startswith("linux-") else COMMON if name.startswith("macos-") else []
            self.assertEqual([n for n in required if n.startswith(PREFIX)], [PREFIX + n for n in expected])
            self.assertEqual(len(required), len(set(required)))

    def test_complete_synthetic_receipt_is_not_runtime_qualification(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], len(REQUIRED) + 1)
        self.assertFalse(result["runtime_qualified"])

    def test_missing_descendant_methods_are_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(missing=name).validate()

    def test_ignored_descendant_methods_are_rejected_even_when_allowed(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(ignored=name).validate()

    def test_failed_descendant_methods_are_rejected_despite_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(failed=name).validate()


if __name__ == "__main__":
    unittest.main()
