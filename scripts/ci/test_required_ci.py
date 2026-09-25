"""Negative controls for the required CI aggregator."""
import copy
import os
import subprocess
import sys
import unittest
from pathlib import Path

from required_ci import REQUIRED_JOBS, validate


class RequiredCiTests(unittest.TestCase):
    def setUp(self):
        self.good = {name: {"result": "success", "outputs": {}}
                     for name in REQUIRED_JOBS}

    def test_complete_success_is_accepted(self):
        validate(self.good)

    def test_each_non_success_job_is_rejected(self):
        for job in REQUIRED_JOBS:
            for result in ("failure", "cancelled", "skipped", "pending", None, True):
                with self.subTest(job=job, result=result):
                    candidate = copy.deepcopy(self.good)
                    candidate[job]["result"] = result
                    with self.assertRaises(ValueError):
                        validate(candidate)

    def test_each_missing_job_is_rejected(self):
        for job in REQUIRED_JOBS:
            candidate = dict(self.good)
            del candidate[job]
            with self.subTest(job=job), self.assertRaises(ValueError):
                validate(candidate)

    def test_extra_jobs_require_explicit_policy_update(self):
        candidate = dict(self.good, unexpected={"result": "success"})
        with self.assertRaises(ValueError):
            validate(candidate)

    def test_malformed_reports_cannot_succeed(self):
        for candidate in (None, [], "success", {}, 1):
            with self.subTest(candidate=candidate), self.assertRaises(ValueError):
                validate(candidate)
        for value in (None, "success", {}, {"outputs": {}}):
            candidate = dict(self.good)
            candidate["windows"] = value
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate(candidate)

    def test_command_rejects_missing_or_invalid_input(self):
        script = Path(__file__).with_name("required_ci.py")
        for value in (None, "{", "[]", "{}"):  # no accidental green on empty input
            env = dict(os.environ)
            env.pop("CI_NEEDS_JSON", None)
            if value is not None:
                env["CI_NEEDS_JSON"] = value
            result = subprocess.run([sys.executable, str(script)], env=env,
                                    capture_output=True, text=True, timeout=10)
            with self.subTest(value=value):
                self.assertEqual(result.returncode, 1)
                self.assertIn("Required CI: FAIL", result.stderr)


if __name__ == "__main__":
    unittest.main()
