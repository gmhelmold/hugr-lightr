"""Destination registration and synthetic evidence controls, not native execution."""
import json
import re
import unittest
from pathlib import Path

import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "store::foundation::destination_tests::"
COMMON = ['destination_precancel_precedes_native_resolution', 'destination_expired_deadline_precedes_native_resolution', 'destination_platform_disposition_is_explicit_without_live_source']
NATIVE = ['destination_existing_handle_is_the_requested_directory', 'destination_checks_each_protected_root_and_all_overlap_directions', 'destination_missing_target_never_exposes_its_existing_ancestor', 'destination_alias_and_parent_use_native_directory_identity', 'destination_revalidation_detects_replacement_without_retargeting_handle', 'destination_missing_protected_root_and_regular_file_are_not_created_or_adopted', 'destination_revalidation_observes_cancellation_and_deadline', 'destination_only_does_not_change_source_only_or_paired_inspection']
REQUIRED = [PREFIX + n for n in COMMON] + [PREFIX + "unix::" + n for n in NATIVE]


class DestinationPolicyTests(unittest.TestCase):
    def fixture(self, missing=None, ignored=None, failed=None):
        sample = fixtures.NativeEvidenceTests()
        sample.setUp()
        self.addCleanup(sample.doCleanups)
        sample.policy["required_tests"]["store"] = [PREFIX + n for n in COMMON]
        sample.policy["profiles"]["native"]["required_tests"] = {
            "store": [PREFIX + "unix::" + n for n in NATIVE]}
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

    def test_registration_preserves_platform_distinction_and_exact_methods(self):
        policy = json.loads((ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text())
        folder = ROOT / "crates/lightr-store/src/store/foundation"
        for filename, expected in [("destination_tests.rs", COMMON), ("destination_unix_tests.rs", NATIVE)]:
            declared = re.findall(r"^fn (destination_[^(]+)\(", (folder / filename).read_text(), re.M)
            self.assertEqual(declared, expected)
        common = policy["required_tests"]["lightr_store"]
        self.assertEqual([n for n in common if n.startswith(PREFIX)], [PREFIX + n for n in COMMON])
        for name, profile in policy["profiles"].items():
            required = profile.get("required_tests", {}).get("lightr_store", [])
            own = [n for n in required if n.startswith(PREFIX)]
            expected = [PREFIX + "unix::" + n for n in NATIVE] if name.startswith(("linux-", "macos-")) else []
            self.assertEqual(own, expected)
            self.assertEqual(len(required), len(set(required)))

    def test_complete_synthetic_receipt_is_baseline_only(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], 12)
        self.assertFalse(result["runtime_qualified"])

    def test_every_missing_destination_witness_is_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(missing=name).validate()

    def test_every_ignored_destination_witness_is_rejected_even_if_allowlisted(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(ignored=name).validate()

    def test_every_failed_destination_witness_is_rejected_despite_zero_command_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(failed=name).validate()


if __name__ == "__main__":
    unittest.main()
