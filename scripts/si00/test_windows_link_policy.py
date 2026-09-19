"""Windows link-plan required witnesses; synthetic receipts do not prove native execution."""
import json
import re
import unittest
from pathlib import Path
import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "store::foundation::windows_link_plan::tests::"
NAMES = ['windows_link_plan_derives_file_explicit_implied_and_root_directory_kinds',
 'windows_link_plan_relative_parent_segments_preserve_original_target_text',
 'windows_link_plan_missing_intermediate_or_final_targets_are_not_guessed',
 'windows_link_plan_chains_self_cycles_and_link_before_parent_are_rejected',
 'windows_link_plan_file_components_cannot_be_walked_through',
 'windows_link_plan_parent_resolution_never_leaves_captured_root',
 'windows_link_plan_rejects_nonportable_target_syntax_without_rewriting',
 'windows_link_plan_checks_names_of_every_entry_even_without_links',
 'windows_link_plan_reserved_devices_and_extensions_are_rejected',
 'windows_link_plan_exact_case_and_unicode_do_not_claim_native_alias_qualification',
 'windows_link_plan_empty_tree_and_long_names_remain_static_only',
 'windows_link_plan_all_entry_orders_preserve_link_order_and_kinds',
 'windows_link_plan_every_checkpoint_is_terminal_and_preserves_input']
NATIVE = ['windows_link_plan_native_creation_keeps_manifest_derived_kind_after_target_removal']
COMMON = [PREFIX + name for name in NAMES]
NATIVE_REQUIRED = [PREFIX + "native::" + name for name in NATIVE]
REQUIRED = COMMON + NATIVE_REQUIRED

class WindowsLinkPolicyTests(unittest.TestCase):
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

    def test_registration_distinguishes_pure_methods_from_windows_native_witness(self):
        policy = json.loads((ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text())
        folder = ROOT / "crates/lightr-store/src/store/foundation"
        for filename, names in [("windows_link_plan_tests.rs", NAMES), ("windows_link_native_tests.rs", NATIVE)]:
            self.assertEqual(re.findall(r"^fn (windows_link_plan_[^(]+)\(", (folder / filename).read_text(), re.M), names)
        self.assertEqual([n for n in policy["required_tests"]["lightr_store"] if n.startswith(PREFIX)], COMMON)
        for name, profile in policy["profiles"].items():
            own = [n for n in profile.get("required_tests", {}).get("lightr_store", []) if n.startswith(PREFIX)]
            self.assertEqual(own, NATIVE_REQUIRED if name.startswith("windows-") else [])

    def test_complete_synthetic_receipt_is_baseline_only(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], 15)
        self.assertFalse(result["runtime_qualified"])

    def test_every_missing_link_witness_is_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(missing=name).validate()

    def test_every_ignored_link_witness_is_rejected_even_if_allowlisted(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(ignored=name).validate()

    def test_every_failed_link_witness_is_rejected_despite_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                self.fixture(failed=name).validate()

if __name__ == "__main__":
    unittest.main()
