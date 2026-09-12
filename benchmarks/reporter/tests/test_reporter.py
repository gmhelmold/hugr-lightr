from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
FIXTURE = ROOT / "benchmarks/reporter/tests/fixtures/raw.jsonl"
REPORTER = ROOT / "benchmarks/reporter/report.py"
VALIDATOR = ROOT / "benchmarks/scripts/validate_contract.py"
SPEC = ROOT / "benchmarks/benchmark-spec.yaml"


class ReporterTests(unittest.TestCase):
    def validator_command(self, raw_path: Path) -> list[str]:
        return [
            sys.executable, str(VALIDATOR), "--input", str(raw_path), "--spec", str(SPEC), "--rounds", "2",
        ]

    def assert_green_fixture(self, raw_path: Path) -> None:
        result = subprocess.run(self.validator_command(raw_path), check=False, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_reports_come_from_checked_in_raw_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "reports"
            result = subprocess.run(
                [sys.executable, str(REPORTER), "--input", str(FIXTURE), "--out", str(output)],
                check=False,
                text=True,
                capture_output=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            csv_text = (output / "summary.csv").read_text(encoding="utf-8")
            payload = json.loads((output / "summary.json").read_text(encoding="utf-8"))
            markdown = (output / "summary.md").read_text(encoding="utf-8")
            self.assertIn("build-single-stage", csv_text)
            self.assertEqual(payload["statistics"], "not_computed")
            self.assertTrue(all(row["p95_elapsed_ms"] == "not_computed" for row in payload["rows"]))
            self.assertIn("Statistics: `not_computed`.", markdown)

    def test_validator_rejects_missing_supported_round_then_restores_green(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            raw_path = Path(temporary) / "raw.jsonl"
            shutil.copyfile(FIXTURE, raw_path)
            command = self.validator_command(raw_path)
            self.assert_green_fixture(raw_path)

            original = raw_path.read_text(encoding="utf-8")
            lines = original.splitlines()
            raw_path.write_text("\n".join(lines[:-1]) + "\n", encoding="utf-8")
            red = subprocess.run(command, check=False, text=True, capture_output=True)
            self.assertNotEqual(red.returncode, 0)
            self.assertIn("missing expected supported record", red.stderr)

            raw_path.write_text(original, encoding="utf-8")
            restored = subprocess.run(command, check=False, text=True, capture_output=True)
            self.assertEqual(restored.returncode, 0, restored.stderr)

    def test_validator_rejects_malformed_duplicate_and_bad_supported_outcomes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            raw_path = Path(temporary) / "raw.jsonl"
            shutil.copyfile(FIXTURE, raw_path)
            original = raw_path.read_text(encoding="utf-8")
            self.assert_green_fixture(raw_path)

            cases = {
                "malformed": original + "not-json\n",
                "duplicate": original + original.splitlines()[1] + "\n",
            }
            records = [json.loads(line) for line in original.splitlines()]
            for outcome in ("failed", "timed_out"):
                mutated = [dict(record) for record in records]
                mutated[1]["outcome"] = outcome
                cases[outcome] = "\n".join(json.dumps(record, sort_keys=True) for record in mutated) + "\n"

            for name, content in cases.items():
                with self.subTest(name=name):
                    raw_path.write_text(content, encoding="utf-8")
                    result = subprocess.run(
                        self.validator_command(raw_path), check=False, text=True, capture_output=True
                    )
                    self.assertNotEqual(result.returncode, 0)

            raw_path.write_text(original, encoding="utf-8")
            self.assert_green_fixture(raw_path)


if __name__ == "__main__":
    unittest.main()
