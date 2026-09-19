"""Registration of file-capture witnesses; these are policy tests, not native runs."""
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
NAMES = ['store::cas::preparation::file_capture::tests::file_capture_copy_only_uses_full_bytes_from_offset_zero', 'store::cas::preparation::file_capture::tests::file_capture_partial_unsupported_clone_falls_back_without_tail', 'store::cas::preparation::file_capture::tests::file_capture_clone_resource_error_is_not_hidden_by_fallback', 'store::cas::preparation::file_capture::tests::file_capture_claimed_clone_success_still_checks_digest_and_length', 'store::cas::preparation::file_capture::tests::file_capture_precancel_does_not_allocate_or_invoke_clone', 'store::cas::preparation::file_capture::tests::file_capture_cancel_after_clone_and_during_hash_cleans_only_owned_file', 'store::cas::preparation::file_capture::tests::file_capture_sync_error_remains_fatal_for_both_capture_modes', 'store::cas::preparation::file_capture::tests::file_capture_empty_source_never_claims_an_unexecuted_clone', 'store::cas::preparation::file_capture::tests::file_capture_native_or_reported_fallback_preserves_readonly_source', 'store::cas::preparation::file_capture::tests::file_capture_lease_composes_with_existing_publication_and_requalification', 'store::cas::preparation::file_capture::tests::file_capture_fallback_classification_rejects_real_io_errors', 'store::cas::preparation::file_capture::edge_tests::file_capture_opened_source_is_not_reinterpreted_by_path', 'store::cas::preparation::file_capture::edge_tests::file_capture_unknown_clone_scratch_blocks_cleanup_and_fallback']


class FileCapturePolicyTests(unittest.TestCase):
    def test_capture_methods_remain_mandatory_unique_and_declared(self):
        policy = json.loads((ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text())
        required = policy["required_tests"]["lightr_store"]
        self.assertEqual(len(required), len(set(required)))
        for name in NAMES:
            with self.subTest(name=name):
                self.assertEqual(required.count(name), 1)
                suffix = "file_capture_edge_tests.rs" if "::edge_tests::" in name else "file_capture_tests.rs"
                source = (ROOT / "crates/lightr-store/src/store/cas" / suffix).read_text()
                self.assertIn("#[test]\nfn " + name.rsplit("::", 1)[1] + "(", source)

    def test_existing_publication_control_is_scoped_to_its_modules(self):
        source = (ROOT / "scripts/si01/publication_controls.py").read_text()
        self.assertIn("PREFIX + 'publication', '--', '--test-threads=2'", source)
        self.assertNotIn("'publication_', '--'", source)
        self.assertIn("29 passed; 0 failed; 0 ignored", source)

    def test_capture_modes_and_native_witness_have_no_ignore(self):
        for file in ["file_capture_tests.rs", "file_capture_edge_tests.rs"]:
            source = (ROOT / "crates/lightr-store/src/store/cas" / file).read_text()
            self.assertNotRegex(source, r"#\[ignore(?:\]|\s|=)")
        source = (ROOT / "crates/lightr-store/src/store/cas/file_capture_tests.rs").read_text()
        self.assertIn('"macOS native witness unavailable: {method:?}"', source)


if __name__ == "__main__":
    unittest.main()
