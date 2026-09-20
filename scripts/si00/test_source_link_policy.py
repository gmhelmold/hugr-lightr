"""Exact native link-text requirements; synthetic receipts are not runtime proof."""
import ast
import json
import re
import sys
import unittest
from pathlib import Path
from evidence import InvalidEvidence
import test_evidence as fixtures

ROOT = Path(__file__).resolve().parents[2]
PREFIX = 'store::foundation::topology::topology_link::tests::'
UNIX = ['source_link_preserves_dangling_and_exact_target_text', 'source_link_reads_file_directory_and_protected_target_without_following', 'source_link_rejects_intermediate_aliases_and_cycles', 'source_link_rejects_non_normal_paths_and_work_budgets', 'source_link_exact_byte_budget_never_accepts_truncation', 'source_link_preserves_missing_and_wrong_type_errors', 'source_link_detects_replacement_after_read', 'source_link_detects_replaced_parent_without_retargeting', 'source_link_detects_original_source_replacement', 'source_link_returned_text_does_not_depend_on_later_target_existence', 'source_link_cancellation_and_deadline_precede_access', 'source_link_cancellation_after_completed_validation_is_not_success', 'source_link_scoped_errors_preserve_original_cause', 'source_link_pinned_handle_is_symlink_and_close_on_exec']
LINUX = ['source_link_linux_raw_name_is_not_lossily_rewritten', 'source_link_linux_non_utf8_target_is_rejected_not_recoded']
REQUIRED = [PREFIX + n for n in UNIX] + [PREFIX + "linux::" + n for n in LINUX]

class SourceLinkPolicyTests(unittest.TestCase):
    def fixture(self, missing=None, ignored=None, failed=None):
        sample = fixtures.NativeEvidenceTests()
        sample.setUp()
        self.addCleanup(sample.doCleanups)
        sample.policy["required_tests"]["store"] = []
        sample.policy["profiles"]["native"]["required_tests"] = {"store": REQUIRED[:]}
        statuses = {n: "ignored" if n == ignored else "FAILED" if n == failed else "ok"
                    for n in REQUIRED if n != missing}
        (sample.root / "list-store.stdout").write_text("".join(n + ": test\n" for n in statuses))
        counts = [sum(v == s for v in statuses.values()) for s in ("ok", "FAILED", "ignored")]
        verdict = "FAILED" if failed else "ok"
        (sample.root / "run-store.stdout").write_text(
            "".join(f"test {n} ... {s}\n" for n,s in statuses.items()) +
            f"test result: {verdict}. {counts[0]} passed; {counts[1]} failed; {counts[2]} ignored; "
            "0 measured; 0 filtered out\n")
        if ignored:
            sample.policy["allowed_ignored"][ignored] = "Synthetic rejection only"
        sample.rehash()
        return sample

    def test_exact_names_and_platform_boundaries(self):
        code = ROOT / "crates/lightr-store/src/store/foundation/topology_link_tests.rs"
        self.assertEqual(re.findall(r"^fn (source_link_\w+)\(", code.read_text(), re.M), UNIX)
        linux = code.with_name("topology_link_linux_tests.rs")
        self.assertEqual(re.findall(r"^fn (source_link_\w+)\(", linux.read_text(), re.M), LINUX)
        policy = json.loads((ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text())
        self.assertFalse(any(n.startswith(PREFIX) for n in policy["required_tests"]["lightr_store"]))
        for name, profile in policy["profiles"].items():
            actual = [n for n in profile.get("required_tests", {}).get("lightr_store", []) if n.startswith(PREFIX)]
            expected = REQUIRED if name.startswith("linux-") else [PREFIX + n for n in UNIX] if name.startswith("macos-") else []
            self.assertEqual(actual, expected)

    def test_all_prior_control_families_and_new_seams_remain_exact(self):
        sys.path.insert(0, str(ROOT / "scripts/si01"))
        try:
            import topology_controls as c
            for name, count in [("CASES",10),("EMPTY_CASES",2),("DESCENDANT_CASES",3),("SCRATCH_CASES",3),("LINK_CASES",4)]:
                self.assertEqual(len(getattr(c,name)), count, name)
            for label, filename, old, _new, test, _message in c.LINK_CASES:
                self.assertEqual((ROOT / c.BASE / filename).read_text().count(old), 1, label)
                self.assertIn(test, REQUIRED)
            source = (ROOT / "scripts/si01/topology_controls.py").read_text()
            loops = [n for n in ast.walk(ast.parse(source)) if isinstance(n, ast.For)
                     and isinstance(n.target, ast.Tuple)
                     and any(isinstance(v, ast.Name) and v.id == "label" for v in n.target.elts)]
            self.assertEqual(len(loops), 1)
            self.assertCountEqual([n.id for n in ast.walk(loops[0].iter) if isinstance(n, ast.Name)],
                                  ["CASES", "EMPTY_CASES", "DESCENDANT_CASES", "SCRATCH_CASES", "LINK_CASES", "LISTING_CASES", "DEST_ANCHOR_CASES", "DEST_REPR_CASES", "DEST_PREPARE_CASES"])
            for stage in ("pristine", "restored"):
                self.assertEqual(source.count(f'run(root, link_suite, "link-{stage}")'), 1)
        finally:
            sys.path.pop(0)

    def test_complete_synthetic_receipt_is_not_native_qualification(self):
        result = self.fixture().validate()
        self.assertEqual(result["passed"], len(REQUIRED) + 1)
        self.assertFalse(result["runtime_qualified"])

    def test_missing_names_are_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaises(InvalidEvidence):
                self.fixture(missing=name).validate()

    def test_ignored_names_are_rejected_even_when_allowlisted(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaises(InvalidEvidence):
                self.fixture(ignored=name).validate()

    def test_failed_names_are_rejected_even_with_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaises(InvalidEvidence):
                self.fixture(failed=name).validate()

if __name__ == "__main__":
    unittest.main()
