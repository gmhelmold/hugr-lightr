"""Compiled structural controls over in-memory trees; no materializer is invoked."""
from __future__ import annotations
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]
BASE = "crates/lightr-store/src/store/foundation/"
PREFIX = "store::foundation::tree_plan::"
CASES = [('duplicates',
  'tree_plan.rs',
  'if pair[0].path() == pair[1].path() {',
  'if false && pair[0].path() == pair[1].path() {',
  'tree_plan_duplicate_entries_of_every_kind_are_rejected',
  'duplicate explicit entry accepted'),
 ('ancestor-kind',
  'tree_plan.rs',
  'if !matches!(entries[index], Entry::Dir { .. }) {',
  'if false && !matches!(entries[index], Entry::Dir { .. }) {',
  'tree_plan_non_directory_ancestors_are_rejected_in_all_entry_orders',
  'non-directory ancestor accepted'),
 ('total-size',
  'tree_plan.rs',
  'if total != manifest.total_size {',
  'if false && total != manifest.total_size {',
  'tree_plan_declared_totals_and_sum_overflow_are_rejected',
  'mismatched total accepted'),
 ('work-budget',
  'tree_plan.rs',
  '.filter(|total| *total <= maximum)',
  '.filter(|total| *total <= maximum || true)',
  'tree_plan_caller_budgets_include_repeated_components_and_link_text',
  'work budget exceeded without failure'),
 ('normal-components',
  'tree_plan.rs',
  'if component.is_empty() || component == "." || component == ".." {',
  'if false && (component.is_empty() || component == "." || component == "..") {',
  'tree_plan_names_are_exact_relative_components_not_normalized_paths',
  'invalid relative components accepted'),
 ('implied-directories',
  'tree_plan.rs',
  'directories.push(&path[..end]);',
  'let _ = &path[..end];',
  'tree_plan_unsorted_tree_preserves_entries_and_includes_implied_directories',
  'assertion `left == right` failed')]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    out = parser.parse_args().out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    git = lambda *args: subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()
    receipt = dict(schema=1, checkout=git("rev-parse", "HEAD"), tree=git("rev-parse", "HEAD^{tree}"),
                   controls=[], production_protocol_enabled=False)

    def require(ok, message):
        if not ok:
            raise RuntimeError(message)

    def run(root, argv, name, limit=600):
        path = out / (name + ".log")
        env = os.environ.copy()
        env.update(CARGO_BUILD_JOBS="2", CARGO_INCREMENTAL="0", CARGO_PROFILE_TEST_DEBUG="0", CARGO_PROFILE_DEV_DEBUG="0")
        with path.open("wb") as log:
            child = subprocess.Popen(argv, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                code = child.wait(timeout=limit)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=30)
                raise RuntimeError("timeout is not a causal kill: " + name)
        return code, path.read_text(errors="replace")

    cargo = ["cargo", "+1.96.0", "test", "--locked", "-p", "lightr-store", "--lib"]
    suite = cargo + [PREFIX, "--", "--nocapture"]
    expected = r"test result: ok\. 15 passed; 0 failed; 0 ignored;"
    try:
        require(not git("status", "--porcelain", "--untracked-files=no"), "tracked source dirty")
        archive = subprocess.check_output(["git", "archive", "--format=zip", "HEAD"], cwd=ROOT)
        (out / "source.zip").write_bytes(archive)
        with tempfile.TemporaryDirectory(prefix="si01-tree-plan-controls-") as temporary:
            root = Path(temporary).resolve()
            with zipfile.ZipFile(io.BytesIO(archive)) as z:
                for item in z.infolist():
                    require((root / item.filename).resolve().is_relative_to(root), "unsafe archive path")
                z.extractall(root)
            code, text = run(root, suite, "pristine")
            require(code == 0 and re.search(expected, text), "pristine suite absent or failed")
            for label, name, old, new, test, message in CASES:
                path = root / BASE / name
                original = path.read_text()
                require(original.count(old) == 1, "mutation seam mismatch: " + label)
                path.write_text(original.replace(old, new, 1))
                try:
                    code, _ = run(root, cargo + ["--no-run"], label + "-build")
                    require(code == 0, "mutant compilation failed: " + label)
                    full = test if test.startswith("store::") else PREFIX + "tests::" + test
                    code, text = run(root, cargo + [full, "--", "--exact"], label + "-test", 60)
                    require(code == 101 and f"test {full} ... FAILED" in text and message in text
                            and re.search(r"test result: FAILED\. 0 passed; 1 failed; 0 ignored;", text),
                            "expected causal assertion absent: " + label)
                    receipt["controls"].append(dict(name=label, build_exit=0, test_exit=101, test=full))
                finally:
                    path.write_text(original)
            code, text = run(root, suite, "restored")
            require(code == 0 and re.search(expected, text), "restored suite failed")
        receipt["status"] = "TREE_PLAN_CONTROLS_PASSED"
    except Exception as error:
        receipt["status"] = "FAILED"
        receipt["error"] = str(error)
    receipt["artifacts"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(out.iterdir()) if p.is_file()}
    (out / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))
    return 0 if receipt["status"] == "TREE_PLAN_CONTROLS_PASSED" else 1


if __name__ == "__main__":
    raise SystemExit(main())
