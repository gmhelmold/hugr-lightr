"""Mandatory listing witnesses; synthetic policy checks do not qualify native Rust."""
import ast
import json
import re
import sys
import unittest
from pathlib import Path
import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]
PREFIX = 'store::foundation::topology::topology_listing::native::tests::'
MAIN_NAMES = [
    'listing_empty_directory_fits_zero_budgets',
    'listing_keeps_hidden_directory_and_dangling_link_names',
    'listing_nested_directory_uses_no_follow_descendant_acquisition',
    'listing_native_entry_and_utf8_byte_budgets_are_exact',
    'listing_ignores_an_offset_previously_advanced_on_the_held_source',
    'listing_revalidates_source_root_after_native_enumeration',
    'listing_revalidates_nested_binding_after_native_enumeration',
    'listing_observations_are_fresh_but_do_not_freeze_contents',
    'listing_precancel_and_expired_deadline_are_terminal',
    'listing_missing_directory_and_file_source_keep_native_errors',
    'listing_non_utf8_native_name_is_an_error_not_an_omission',
]
COLLECTION_NAMES = [
    'listing_read_failure_after_a_name_is_not_partial_success',
    'listing_cancellation_after_eof_rejects_the_result',
    'listing_duplicates_and_malformed_names_are_not_silently_omitted',
    'listing_budget_failure_stops_without_scanning_the_remaining_names',
    'listing_empty_stream_and_single_dot_variants_are_accepted',
    'listing_checked_close_rejects_success_and_preserves_prior_error',
]
COMMON = [PREFIX + n for n in MAIN_NAMES[:-1]] + [PREFIX + 'collection::' + n for n in COLLECTION_NAMES]
LINUX = [PREFIX + MAIN_NAMES[-1]]
REQUIRED = COMMON + LINUX
FAMILIES = ['CASES', 'EMPTY_CASES', 'DESCENDANT_CASES', 'SCRATCH_CASES', 'LINK_CASES', 'LISTING_CASES', 'DEST_ANCHOR_CASES', 'DEST_REPR_CASES', 'DEST_PREPARE_CASES']


class SourceListingPolicyTests(unittest.TestCase):
    def fixture(self, missing=None, ignored=None, failed=None):
        sample = fixtures.NativeEvidenceTests()
        sample.setUp()
        self.addCleanup(sample.doCleanups)
        sample.policy['required_tests']['store'] = []
        sample.policy['profiles']['native']['required_tests'] = {'store': REQUIRED[:]}
        statuses = {n: 'ignored' if n == ignored else 'FAILED' if n == failed else 'ok'
                    for n in REQUIRED if n != missing}
        (sample.root / 'list-store.stdout').write_text(''.join(n + ': test\n' for n in statuses))
        counts = [sum(v == status for v in statuses.values()) for status in ('ok', 'FAILED', 'ignored')]
        verdict = 'FAILED' if failed else 'ok'
        (sample.root / 'run-store.stdout').write_text(
            ''.join(f'test {n} ... {status}\n' for n, status in statuses.items()) +
            f'test result: {verdict}. {counts[0]} passed; {counts[1]} failed; {counts[2]} ignored; '
            '0 measured; 0 filtered out\n')
        if ignored:
            sample.policy['allowed_ignored'][ignored] = 'Synthetic rejection only'
        sample.rehash()
        return sample

    def test_exact_names_and_platform_boundaries_are_registered(self):
        policy = json.loads((ROOT / 'docs/plans/snapshot-integrity/bootstrap/expected-tests.json').read_text())
        base = ROOT / 'crates/lightr-store/src/store/foundation'
        for filename, expected in [('topology_listing_tests.rs', MAIN_NAMES),
                                   ('topology_listing_collect_tests.rs', COLLECTION_NAMES)]:
            actual = re.findall(r'^fn (listing_\w+)\(', (base / filename).read_text(), re.M)
            self.assertEqual(actual, expected)
        self.assertEqual(len(REQUIRED), 17)
        self.assertFalse(any(n.startswith(PREFIX) for n in policy['required_tests']['lightr_store']))
        for name, profile in policy['profiles'].items():
            required = profile.get('required_tests', {}).get('lightr_store', [])
            expected = REQUIRED if name.startswith('linux-') else COMMON if name.startswith('macos-') else []
            self.assertEqual([n for n in required if n.startswith(PREFIX)], expected)
            self.assertEqual(len(required), len(set(required)))

    def test_listing_controls_keep_prior_families_and_separate_suites(self):
        sys.path.insert(0, str(ROOT / 'scripts/si01'))
        try:
            import topology_controls as controls
            for family, count in zip(FAMILIES, [10, 2, 3, 3, 4, 4, 3]):
                self.assertEqual(len(getattr(controls, family)), count, family)
            for label, filename, old, _new, test, _message in controls.LISTING_CASES:
                self.assertEqual((ROOT / controls.BASE / filename).read_text().count(old), 1, label)
                self.assertIn(test, REQUIRED)
            text = (ROOT / 'scripts/si01/topology_controls.py').read_text()
            loops = [n for n in ast.walk(ast.parse(text)) if isinstance(n, ast.For)
                     and isinstance(n.target, ast.Tuple)
                     and any(isinstance(v, ast.Name) and v.id == 'label' for v in n.target.elts)]
            self.assertEqual(len(loops), 1)
            self.assertCountEqual([n.id for n in ast.walk(loops[0].iter) if isinstance(n, ast.Name)], FAMILIES)
            self.assertIn('listing_expected = rf"test result: ok\\. {17 if linux else 16} passed;', text)
            for stage in ['pristine', 'restored']:
                self.assertEqual(text.count(f'run(root, listing_suite, "listing-{stage}")'), 1)
        finally:
            sys.path.pop(0)

    def test_complete_synthetic_receipt_is_not_runtime_qualification(self):
        result = self.fixture().validate()
        self.assertEqual(result['passed'], len(REQUIRED) + 1)
        self.assertFalse(result['runtime_qualified'])

    def test_missing_listing_methods_are_rejected(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, 'required smoke'):
                self.fixture(missing=name).validate()

    def test_ignored_listing_methods_are_rejected_even_when_allowed(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, 'required smoke'):
                self.fixture(ignored=name).validate()

    def test_failed_listing_methods_are_rejected_despite_zero_exit(self):
        for name in REQUIRED:
            with self.subTest(name=name), self.assertRaisesRegex(InvalidEvidence, 'required smoke'):
                self.fixture(failed=name).validate()


if __name__ == '__main__':
    unittest.main()
