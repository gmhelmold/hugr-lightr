"""Inspection-name enforcement; all receipt fixtures here are synthetic.

No native binary runs in these unit controls. The real five-profile workflow
must separately execute and validate these required Rust method names.
"""
import json
import unittest
from pathlib import Path

import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
MODULE = "store::foundation::inspection_cancellation_tests::"
REQUIRED = [
    MODULE + "inspection_cancelled_at_entry_stops_before_receipt_work",
    MODULE + "inspection_preserves_real_io_error_when_cancellation_also_occurs",
    MODULE + "inspection_checks_between_real_payload_blocks",
    MODULE + "inspection_cancellation_at_empty_payload_eof_is_terminal",
    MODULE + "inspection_precancel_and_expired_deadline_precede_receipt_decoding",
    MODULE + "inspection_uncancelled_keeps_verification_and_returns_rewound_file",
    "store::foundation::inspection_waiter_tests::"
    "inspection_cancellation_wakes_waiter_and_preserves_distinct_progress",
]


class InspectionPolicyTests(unittest.TestCase):
    def fixture(self, missing=None, ignored=None, failed=None):
        fixture = fixtures.NativeEvidenceTests()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        fixture.policy["required_tests"]["store"] = REQUIRED.copy()
        present = [name for name in REQUIRED if name != missing]
        (fixture.root / "list-store.stdout").write_text(
            "".join(name + ": test\n" for name in present), encoding="utf-8")
        statuses = {name: ("ignored" if name == ignored else
                           "FAILED" if name == failed else "ok")
                    for name in present}
        passed_count = sum(value == "ok" for value in statuses.values())
        ignored_count = sum(value == "ignored" for value in statuses.values())
        failed_count = sum(value == "FAILED" for value in statuses.values())
        # All logs and hashes agree; the independent required-name policy must
        # still reject absence/non-success. Command exit stays zero on purpose.
        verdict = "FAILED" if failed_count else "ok"
        (fixture.root / "run-store.stdout").write_text(
            "".join(f"test {name} ... {status}\n"
                    for name, status in statuses.items()) +
            f"test result: {verdict}. {passed_count} passed; "
            f"{failed_count} failed; {ignored_count} ignored; "
            "0 measured; 0 filtered out\n", encoding="utf-8")
        if ignored:
            fixture.policy["allowed_ignored"][ignored] = "Synthetic control only"
        fixture.rehash()
        return fixture

    def test_inspection_names_are_mandatory_and_unique(self):
        policy = json.loads((ROOT / "docs/plans/snapshot-integrity/bootstrap/"
                             "expected-tests.json").read_text(encoding="utf-8"))
        names = policy["required_tests"]["lightr_store"]
        self.assertEqual(len(names), len(set(names)))
        for full_name in REQUIRED:
            with self.subTest(name=full_name):
                self.assertEqual(names.count(full_name), 1,
                                 "inspection witness is not mandatory")
                module, name = full_name.rsplit("::", 1)
                path = ROOT / "crates/lightr-store/src/store/foundation" / (
                    module.rsplit("::", 1)[1] + ".rs")
                self.assertIn("#[test]\nfn " + name + "(",
                              path.read_text(encoding="utf-8"))

    def test_complete_fixture_is_only_baseline_acceptance(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], 8)  # Seven store + one index fixture.
        self.assertFalse(result["runtime_qualified"])

    def test_each_missing_required_inspection_is_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name):
                with self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                    self.fixture(missing=name).validate()

    def test_each_ignored_required_inspection_is_rejected_even_if_allowlisted(self):
        for name in REQUIRED:
            with self.subTest(name=name):
                with self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                    self.fixture(ignored=name).validate()

    def test_each_failed_required_inspection_is_rejected_despite_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name):
                with self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                    self.fixture(failed=name).validate()


if __name__ == "__main__":
    unittest.main()
