"""Additive planned-root registration; synthetic controls are not native execution."""
import ast
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
PREFIX = "store::foundation::planned_roots_tests::"


class PlannedRootsPolicyTests(unittest.TestCase):
    def setUp(self):
        self.policy = json.loads((ROOT / "docs/plans/snapshot-integrity/bootstrap/expected-tests.json").read_text())
        folder = ROOT / "crates/lightr-store/src/store/foundation"
        self.common = re.findall(r"^fn (planned_roots_[^(]+)\(", (folder / "planned_roots_tests.rs").read_text(), re.M)
        self.native = re.findall(r"^fn (planned_roots_[^(]+)\(", (folder / "planned_roots_unix_tests.rs").read_text(), re.M)

    def test_common_methods_are_mandatory_on_every_profile(self):
        self.assertEqual(len(self.common), 3)
        expected = [PREFIX + name for name in self.common]
        actual = [n for n in self.policy["required_tests"]["lightr_store"] if n.startswith(PREFIX)]
        self.assertEqual(actual, expected)

    def test_unix_methods_are_required_only_on_qualified_profiles(self):
        self.assertEqual(len(self.native), 14)
        for name, profile in self.policy["profiles"].items():
            actual = [n for n in profile.get("required_tests", {}).get("lightr_store", []) if n.startswith(PREFIX)]
            expected = [PREFIX + "unix::" + n for n in self.native] if name.startswith(("linux-", "macos-")) else []
            with self.subTest(profile=name):
                self.assertEqual(actual, expected)

    def test_each_topology_mutation_has_one_exact_source_seam(self):
        path = ROOT / "scripts/si01/topology_controls.py"
        declaration = next(node for node in ast.parse(path.read_text()).body
                           if isinstance(node, ast.Assign) and any(
                               isinstance(target, ast.Name) and target.id == "CASES"
                               for target in node.targets))
        cases = ast.literal_eval(declaration.value)
        self.assertEqual(len(cases), 10)
        for label, name, old, _new, _test, _message in cases:
            source = (ROOT / "crates/lightr-store/src/store/foundation" / name).read_text()
            with self.subTest(control=label):
                self.assertEqual(source.count(old), 1)

    def test_causal_controls_have_exact_planned_and_preserved_suite_selectors(self):
        text = (ROOT / "scripts/si01/topology_controls.py").read_text()
        for selector, count in [("PREFIX", 20), ('"store::foundation::destination_tests::"', 11),
                                ('"store::foundation::planned_roots_tests::"', 17)]:
            self.assertIn(selector, text)
            self.assertIn(f"{count} passed; 0 failed; 0 ignored;", text)
        for label in ["planned-root-resolution", "planned-root-native-ambiguity", "planned-root-public-ancestor"]:
            self.assertEqual(text.count('"' + label + '"'), 1)


if __name__ == "__main__":
    unittest.main()
