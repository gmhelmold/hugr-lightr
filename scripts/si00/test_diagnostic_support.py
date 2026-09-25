"""Test diagnostic fixtures/classification, not a substitute for CLI execution."""
import gzip
import tempfile
import unittest
from pathlib import Path

from diagnose_capture import classify, make_case
from fixtures import tree
from goldens import golden_bytes, MAX_BYTES


class DiagnosticSupportTests(unittest.TestCase):
    def test_exit_zero_with_changed_tree_is_not_pass(self):
        self.assertEqual(classify({'exit': 0}, {'exit': 0}, [{'target': 'a\\b'}], [{'target': 'a/b'}]), 'TREE_MISMATCH')
        self.assertEqual(classify({'exit': 1}, None, [], None), 'SNAPSHOT_REJECTED')
        self.assertEqual(classify({'exit': 0}, {'exit': 1}, [], None), 'HYDRATE_REJECTED')
        self.assertEqual(classify({'exit': 0}, {'exit': 0}, [], []), 'ROUNDTRIP_MATCH')

    def test_ordinary_fixture_has_independent_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / 'source'
            make_case(source, 'ordinary')
            self.assertEqual((source / 'payload').read_bytes(), b'si00-diagnostic\x00\xff\n')
            self.assertEqual(len(tree(source)), 1)
            with self.assertRaises(FileExistsError):
                make_case(source, 'ordinary')

    def test_unknown_case_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(ValueError, 'unknown'):
                make_case(Path(tmp) / 'source', 'unknown')

    def test_golden_missing_changed_and_oversized_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / 'bad.json'
            with self.assertRaises(FileNotFoundError): golden_bytes(p)
            for data in (b'', b'x' * MAX_BYTES, b'x' * (MAX_BYTES + 1)):
                p.write_bytes(data)
                with self.assertRaises(ValueError): golden_bytes(p)
            p = Path(tmp) / 'bad.json.gz'
            p.write_bytes(gzip.compress(b'x' * (MAX_BYTES * 10), mtime=0))
            with self.assertRaises(ValueError): golden_bytes(p)


if __name__ == '__main__':
    unittest.main()
