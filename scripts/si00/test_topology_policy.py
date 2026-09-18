"""Profile policy controls use synthetic receipts, not native filesystem runs."""
import json
import re
import unittest
from pathlib import Path
import test_evidence as fixtures
from evidence import InvalidEvidence

ROOT = Path(__file__).resolve().parents[2]

class TopologyPolicyTests(unittest.TestCase):
    def sample(self, status="ok", absent=False):
        fixture = fixtures.NativeEvidenceTests()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        fixture.policy["profiles"]["native"]["required_tests"] = {"store": ["platform_witness"]}
        fixture.policy["allowed_ignored"]["platform_witness"] = "Synthetic rejection case"
        if not absent:
            (fixture.root / "list-store.stdout").write_text("test_one: test\nplatform_witness: test\n")
            passed = 1 + (status == "ok")
            ignored = int(status == "ignored")
            failed = int(status == "FAILED")
            verdict = "FAILED" if failed else "ok"
            (fixture.root / "run-store.stdout").write_text(
                "test test_one ... ok\n" + f"test platform_witness ... {status}\n" +
                f"test result: {verdict}. {passed} passed; {failed} failed; {ignored} ignored; "
                "0 measured; 0 filtered out\n")
        fixture.rehash()
        return fixture

    def test_platform_requirements_accept_complete_and_reject_absent(self):
        self.assertEqual(self.sample().validate()["passed"], 3)
        with self.assertRaisesRegex(InvalidEvidence, "required smoke"):
            self.sample(absent=True).validate()

    def test_profile_ignored_and_failed_are_not_success_despite_allowlist_and_zero_exit(self):
        for status in ["ignored", "FAILED"]:
            with self.subTest(status=status):
                with self.assertRaisesRegex(InvalidEvidence, "required smoke"):
                    self.sample(status=status).validate()

    def test_unknown_malformed_or_duplicate_profile_requirements_fail(self):
        for extra in [{"unknown": ["x"]}, {"store": "x"}, {"store": [""]},
                      {"store": ["test_one"]}, {"store": ["platform_witness", "platform_witness"]}]:
            with self.subTest(extra=extra):
                sample = self.sample()
                sample.policy["profiles"]["native"]["required_tests"] = extra
                with self.assertRaises(InvalidEvidence): sample.validate()

    def test_platform_policy_matches_existing_method_declarations(self):
        policy = json.loads((ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text())
        folder = ROOT / "crates/lightr-store/src/store/foundation"
        prefix = "store::foundation::topology::tests::"
        common = re.findall(r"^fn (topology_[^(]+)\(", (folder / "topology_tests.rs").read_text(), re.M)
        native = re.findall(r"^fn (topology_[^(]+)\(", (folder / "topology_unix_tests.rs").read_text(), re.M)
        self.assertEqual(len(common), 3)
        self.assertEqual(len(native), 16)
        for name in common:
            self.assertEqual(policy["required_tests"]["lightr_store"].count(prefix + name), 1)
        for profile, configuration in policy["profiles"].items():
            specific = configuration.get("required_tests", {}).get("lightr_store", [])
            if profile.startswith(("linux-", "macos-")):
                self.assertEqual(set(specific), {prefix + "unix::" + name for name in native})
                self.assertEqual(len(specific), 16)
            else:
                self.assertEqual(specific, [])

if __name__ == "__main__": unittest.main()
