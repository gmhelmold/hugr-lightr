"""Guard the concrete pre-merge configuration and non-vacuous socket witness."""
import re
import unittest
from pathlib import Path

from required_ci import REQUIRED_JOBS
from repeat_socket_witness import CASE, exact_pass

ROOT = Path(__file__).resolve().parents[2]


class WorkflowContractTests(unittest.TestCase):
    def test_complete_ci_targets_both_merge_destinations(self):
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertRegex(text, r"pull_request:\s+branches: \[main, fix/snapshot-integrity\]")
        self.assertIn('RUSTFLAGS: "-D warnings"', text)

    def test_required_gate_needs_exact_declared_verification_jobs(self):
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        tail = text.split("  required-ci:", 1)[1].split("  release:", 1)[0]
        match = re.search(r"needs: \[([^]]+)\]", tail)
        self.assertIsNotNone(match)
        self.assertEqual({name.strip() for name in match.group(1).split(",")}, REQUIRED_JOBS)
        self.assertIn("if: ${{ always() }}", tail)
        self.assertIn("CI_NEEDS_JSON: ${{ toJSON(needs) }}", tail)
        self.assertIn("python3 scripts/ci/required_ci.py", tail)
        self.assertIn("if: false  # Disabled until GTW gate signed", text)

    def test_native_profiles_treat_warnings_as_errors(self):
        text = (ROOT / ".github/workflows/si00-bootstrap.yml").read_text()
        native = text.split("  native-baseline:", 1)[1].split("  native-evidence-gate:", 1)[0]
        self.assertIn("RUSTFLAGS: '-D warnings'", native)

    def test_intel_and_current_arm_are_distinct_required_environments(self):
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        intel = text.split("  macos-x86_64:", 1)[1].split("  macos-current-arm64:", 1)[0]
        current = text.split("  macos-current-arm64:", 1)[1].split("  macos-arm64:", 1)[0]
        self.assertIn("runs-on: macos-15-intel", intel)
        self.assertIn('test "$(uname -m)" = x86_64', intel)
        self.assertIn("runs-on: macos-latest", current)
        self.assertIn('test "$(uname -m)" = arm64', current)
        self.assertIn("python3 scripts/ci/repeat_socket_witness.py", current)
        self.assertTrue({"macos-x86_64", "macos-current-arm64", "macos-arm64"} <= REQUIRED_JOBS)

    def test_macos_warnings_do_not_erase_configured_swift_rpath(self):
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        for job in ("macos-x86_64", "macos-current-arm64", "macos-arm64"):
            part = text.split("  " + job + ":", 1)[1].split("    steps:", 1)[0]
            with self.subTest(job=job):
                self.assertIn('RUSTFLAGS: "-D warnings -C link-arg=-Wl,-rpath,/usr/lib/swift"', part)

    def test_socket_evidence_requires_one_exact_success(self):
        text = "test " + CASE + " ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 264 filtered out\n"
        self.assertTrue(exact_pass(0, text))
        for candidate in ("", text.replace(CASE, "other_case"), text.replace("1 passed", "0 passed"),
                          text.replace(" ... ok", " ... ignored"), text.replace("0 ignored", "1 ignored")):
            with self.subTest(candidate=candidate):
                self.assertFalse(exact_pass(0, candidate))
        self.assertFalse(exact_pass(101, text))


if __name__ == "__main__":
    unittest.main()
