"""Portable controls for the native-suite gate; not native PTY qualification."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import pty_full_suite as suite

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


class PtySuiteFailureEvidenceTests(unittest.TestCase):
    """Synthetic child outcomes only; these are not Darwin runtime tests."""

    def run_scripted_child(self, outcomes, kill_error=None, spawn_error=None):
        command = ["owned-fixture-not-executed"]
        child = mock.Mock(pid=12345)
        child.wait.side_effect = outcomes
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = root / "failure.log"
            with mock.patch.object(suite.subprocess, "Popen", return_value=child,
                                   side_effect=spawn_error) as spawn, \
                    mock.patch.object(suite.os, "killpg", side_effect=kill_error) as kill:
                record = execute(command, root, {}, log, 0.1)
            self.assertEqual(record["command"], command)
            self.assertEqual(record["sha256"], hashlib.sha256(log.read_bytes()).hexdigest())
            self.assertEqual(record["log_snapshot_bytes"], log.stat().st_size)
            self.assertEqual(spawn.call_count, 1)
        return record, child, kill

    def test_cleanup_timeout_still_returns_failed_command_record(self):
        record, child, kill = self.run_scripted_child([
            subprocess.TimeoutExpired("fixture", 0.1),
            subprocess.TimeoutExpired("fixture", 30),
        ])
        self.assertTrue(record["timed_out"])
        self.assertIsNone(record["exit"])
        self.assertFalse(record["reaped"])
        self.assertEqual(record["errors"]["reap"]["type"], "TimeoutExpired")
        self.assertEqual(child.wait.call_args_list,
                         [mock.call(timeout=0.1), mock.call(timeout=30)])
        kill.assert_called_once_with(child.pid, suite.signal.SIGKILL)

    def test_cleanup_permission_failure_is_recorded_without_claiming_reap(self):
        record, _, _ = self.run_scripted_child([
            subprocess.TimeoutExpired("fixture", 0.1),
            subprocess.TimeoutExpired("fixture", 30),
        ], kill_error=PermissionError(1, "synthetic denied"))
        self.assertTrue(record["timed_out"])
        self.assertFalse(record["reaped"])
        self.assertEqual(record["errors"]["terminate"]["errno"], 1)
        self.assertIn("reap", record["errors"])
        self.assertIsNone(record["exit"])

    def test_process_exit_during_timeout_cleanup_is_not_success(self):
        record, _, _ = self.run_scripted_child([
            subprocess.TimeoutExpired("fixture", 0.1), 0,
        ], kill_error=ProcessLookupError())
        self.assertTrue(record["reaped"])
        self.assertTrue(record["timed_out"])
        self.assertEqual(record["exit"], 0)
        with self.assertRaises(ValueError):
            validate_full_suite(record, "", [ORIGINAL, REGRESSION])

    def test_cleanup_wait_error_preserves_initial_timeout(self):
        record, _, _ = self.run_scripted_child([
            subprocess.TimeoutExpired("fixture", 0.1), OSError(5, "synthetic wait error"),
        ])
        self.assertTrue(record["timed_out"])
        self.assertFalse(record["reaped"])
        self.assertEqual(record["errors"]["reap"]["errno"], 5)

    def test_spawn_failure_records_error_without_sending_signal(self):
        record, child, kill = self.run_scripted_child(
            [], spawn_error=FileNotFoundError(2, "synthetic missing executable"))
        self.assertFalse(record["timed_out"])
        self.assertFalse(record["reaped"])
        self.assertIsNone(record["exit"])
        self.assertEqual(record["errors"]["spawn"]["errno"], 2)
        child.wait.assert_not_called()
        kill.assert_not_called()

    def test_progress_distinguishes_missing_result_from_long_running(self):
        names = [ORIGINAL, REGRESSION, "not_started_or_not_finished"]
        text = (f"test {ORIGINAL} ... ok\n"
                f"test {REGRESSION} has been running for over 60 seconds\n")
        progress = suite.test_progress(text, names)
        self.assertEqual(progress["passed"], [ORIGINAL])
        self.assertEqual(progress["without_result"], names[1:])
        self.assertEqual(progress["long_running"], [REGRESSION])
        self.assertEqual(progress["failed"], [])
        self.assertEqual(progress["ignored"], [])

    def test_failed_main_retains_full_suite_command_and_progress(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "fixture-binary"
            binary.write_bytes(b"not executed")
            out = root / "evidence"
            def fake_execute(command, _root, _env, log, limit):
                stage = log.stem
                text = ("".join(n + ": test\n" for n in [ORIGINAL, REGRESSION, *DESCRIPTOR_TESTS]) if stage == "inventory"
                        else f"test {ORIGINAL} ... ok\n" if stage == "full-cri-suite" else "")
                log.write_text(text)
                failed = stage == "full-cri-suite"
                return {"command": command, "exit": None if failed else 0,
                        "timed_out": failed, "log": log.name,
                        "sha256": hashlib.sha256(log.read_bytes()).hexdigest(),
                        "reaped": not failed, "errors": {"reap": {"type": "TimeoutExpired"}} if failed else {}}
            with mock.patch.object(suite, "ROOT", root), \
                    mock.patch.object(sys, "argv", ["pty_full_suite.py", "--out", str(out)]), \
                    mock.patch.object(suite.platform, "system", return_value="Darwin"), \
                    mock.patch.object(suite.platform, "platform", return_value="synthetic Darwin fixture"), \
                    mock.patch.object(suite.platform, "machine", return_value="synthetic"), \
                    mock.patch.object(suite.subprocess, "check_output", side_effect=["head", "tree", "compiler"]), \
                    mock.patch.object(suite, "select_executable", return_value=binary), \
                    mock.patch.object(suite, "execute", side_effect=fake_execute), \
                    mock.patch("builtins.print"):
                self.assertEqual(suite.main(), 1)
            receipt = json.loads((out / "receipt.json").read_text())
            self.assertEqual(receipt["status"], "FAILED")
            self.assertEqual(len(receipt["commands"]), 3)
            self.assertIsNone(receipt["commands"][-1]["exit"])
            self.assertEqual(receipt["test_progress"]["without_result"], [REGRESSION, *DESCRIPTOR_TESTS])

    def test_process_errors_cannot_be_accepted_even_with_zero_exit(self):
        record, _, _ = self.run_scripted_child([OSError(5, "synthetic initial wait error"), 0])
        self.assertTrue(record["reaped"])
        self.assertFalse(record["timed_out"])
        self.assertEqual(record["errors"]["wait"]["errno"], 5)
        names = [ORIGINAL, REGRESSION]
        text = "".join(f"test {name} ... ok\n" for name in names)
        text += "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"
        with self.assertRaises(ValueError):
            validate_full_suite(record, text, names)


if __name__ == "__main__":
    unittest.main()
