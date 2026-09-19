"""Portable controls for the native-suite gate; not native PTY qualification."""
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

from pty_full_suite import (ORIGINAL, REGRESSION, DESCRIPTOR_TESTS, execute, listed_tests,
                            select_executable, validate_full_suite)

ROOT = Path(__file__).resolve().parents[2]


class PtyFullSuiteTests(unittest.TestCase):
    def setUp(self):
        self.names = [ORIGINAL, REGRESSION, "other::existing_test", *DESCRIPTOR_TESTS]
        self.record = {"exit": 0, "timed_out": False}
        self.text = "".join(f"test {n} ... ok\n" for n in self.names)
        self.text += f"test result: ok. {len(self.names)} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n"

    def test_complete_suite_and_inventory_are_accepted(self):
        self.assertEqual(listed_tests("".join(n + ": test\n" for n in self.names)), self.names)
        validate_full_suite(self.record, self.text, self.names)

    def test_missing_duplicate_or_filtered_methods_are_rejected(self):
        for text in (self.text.replace("test other::existing_test ... ok\n", ""),
                     self.text + f"test {ORIGINAL} ... ok\n",
                     self.text.replace("0 filtered out", "1 filtered out"),
                     self.text.replace("0 ignored", "1 ignored"),
                     self.text.replace("0 failed", "1 failed")):
            with self.subTest(text=text), self.assertRaises(ValueError):
                validate_full_suite(self.record, text, self.names)

    def test_timeout_or_nonzero_exit_cannot_be_a_green_log(self):
        for record in ({"exit": 0, "timed_out": True}, {"exit": 101, "timed_out": False}):
            with self.subTest(record=record), self.assertRaises(ValueError):
                validate_full_suite(record, self.text, self.names)

    def test_inventory_requires_original_and_new_witnesses_once(self):
        for names in ([], [ORIGINAL], self.names + [REGRESSION]):
            with self.subTest(names=names), self.assertRaises(ValueError):
                listed_tests("".join(n + ": test\n" for n in names))

    def test_only_the_owned_compiled_executable_is_selected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "target" / "native-test"
            binary.parent.mkdir()
            binary.write_bytes(b"synthetic metadata; never executed")
            item = {"reason": "compiler-artifact", "target": {"name": "lightr_cri_backend"},
                    "profile": {"test": True}, "executable": str(binary)}
            self.assertEqual(select_executable(json.dumps(item), root), binary.resolve())
            for text in ("{}", json.dumps(item) + "\n" + json.dumps(item)):
                with self.assertRaises(ValueError):
                    select_executable(text, root)
            item["executable"] = str(root / "outside-target")
            with self.assertRaises(ValueError):
                select_executable(json.dumps(item), root)

    @unittest.skipUnless(os.name == "posix", "POSIX process-group watchdog contract")
    def test_watchdog_records_timeout_and_keeps_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            # Timeout need not let the interpreter start: empty output is valid evidence.
            command = [sys.executable, "-c", "import time; time.sleep(30)"]
            record = execute(command, root, dict(os.environ), root / "timeout.log", 0.1)
            self.assertTrue(record["timed_out"])
            self.assertNotEqual(record["exit"], 0)
            self.assertTrue((root / "timeout.log").is_file())
            self.assertEqual(record["limit_seconds"], 0.1)

    def test_completed_command_records_exit_and_preserves_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            command = [sys.executable, "-c", "print('captured output', flush=True)"]
            record = execute(command, root, dict(os.environ), root / "output.log", 10)
            self.assertFalse(record["timed_out"])
            self.assertEqual(record["exit"], 0)
            self.assertEqual((root / "output.log").read_text(), "captured output\n")

    def test_each_descriptor_witness_is_mandatory(self):
        for missing in DESCRIPTOR_TESTS:
            names = [name for name in self.names if name != missing]
            with self.subTest(missing=missing), self.assertRaises(ValueError):
                listed_tests("".join(name + ": test\n" for name in names))

    def test_native_matrix_contains_all_complete_ci_macos_versions(self):
        text = (ROOT / ".github/workflows/pty-drain.yml").read_text()
        self.assertIn("runner: [macos-15-intel, macos-15, macos-14, macos-latest]", text)
        self.assertIn("python3 scripts/ci/pty_full_suite.py --out pty-full-suite", text)
        self.assertIn("python3 scripts/ci/pty_controls.py --out pty-evidence", text)
        self.assertIn("if: always()", text)
        self.assertNotIn("continue-on-error", text)
        self.assertIn("scripts/ci/test_pty_full_suite.py", text)


if __name__ == "__main__":
    unittest.main()
