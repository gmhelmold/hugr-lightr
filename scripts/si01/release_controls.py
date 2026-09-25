"""NativeLock owner-release controls; disposable source only, no live Store.

The six tests must execute without skips. Each mutant must compile and fail
exactly its intended assertion; timeout/build error is never a causal kill.
"""
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
SOURCE = "crates/lightr-store/src/store/foundation/lease_io.rs"
PREFIX = "store::foundation::lease_io::"
CASES = [
    ("close-only", "let _ = File::unlock(&self._file);", "let _ = &self._file;",
     "release_tests::lease_drop_releases_even_with_a_duplicated_os_handle",
     "owner Drop must explicitly release its lock despite a duplicated descriptor"),
    ("foreign-process-unlock", "if self.owner_process == std::process::id() {", "if true {",
     "owner_release_tests::cleanup_with_a_different_owner_process_does_not_unlock_the_live_owner",
     "called `Result::unwrap_err()` on an `Ok` value"),
]

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    git = lambda *a: subprocess.check_output(["git", *a], cwd=ROOT, text=True).strip()
    receipt = {"schema": 1, "checkout_sha": git("rev-parse", "HEAD"),
               "tree": git("rev-parse", "HEAD^{tree}"), "controls": [],
               "production_protocol_enabled": False, "runtime_qualified": False}
    def need(ok, reason):
        if not ok:
            raise RuntimeError(reason)
    def run(argv, root, label, timeout=600):
        log = out / (label + ".log")
        with log.open("wb") as stream:
            proc = subprocess.Popen(argv, cwd=root, stdout=stream,
                                    stderr=subprocess.STDOUT, start_new_session=True)
            try:
                proc.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait(timeout=30)
                raise RuntimeError("watchdog is not a causal kill: " + label)
        return proc.returncode, log.read_text(errors="replace")
    try:
        need(not git("status", "--porcelain", "--untracked-files=no"), "tracked inputs dirty")
        archive = subprocess.check_output(["git", "archive", "--format=zip", "HEAD"], cwd=ROOT)
        (out / "source.zip").write_bytes(archive)
        with tempfile.TemporaryDirectory(prefix="si01-release-controls-") as temporary:
            root = Path(temporary).resolve()
            with zipfile.ZipFile(io.BytesIO(archive)) as z:
                for entry in z.infolist():
                    need((root / entry.filename).resolve().is_relative_to(root), "unsafe archive path")
                z.extractall(root)
            suite = ["cargo", "+1.96.0", "test", "--locked", "-p", "lightr-store",
                     "--lib", PREFIX, "--", "--test-threads=4"]
            code, text = run(suite, root, "pristine")
            expected = r"test result: ok\. 6 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out"
            need(code == 0 and re.search(expected, text), "missing/failed six-test release suite")
            for label, old, new, test, assertion in CASES:
                path = root / SOURCE
                original = path.read_text()
                need(original.count(old) == 1, "mutation seam drift: " + label)
                path.write_text(original.replace(old, new, 1))
                try:
                    code, _ = run(["cargo", "+1.96.0", "test", "--locked", "-p",
                                   "lightr-store", "--lib", "--no-run"], root, label + "-build")
                    need(code == 0, "mutant build failed: " + label)
                    code, text = run(["cargo", "+1.96.0", "test", "--locked", "-p",
                                      "lightr-store", "--lib", PREFIX + test, "--", "--exact"],
                                     root, label + "-test", 60)
                    need(code == 101 and "test " + PREFIX + test + " ... FAILED" in text
                         and assertion in text
                         and re.search(r"test result: FAILED\. 0 passed; 1 failed; 0 ignored;", text),
                         "wrong/absent assertion: " + label)
                    receipt["controls"].append({"id": label, "test": PREFIX + test,
                                                "build_exit": 0, "test_exit": 101,
                                                "status": "KILLED_CAUSALLY"})
                finally:
                    path.write_text(original)
            code, text = run(suite, root, "restored")
            need(code == 0 and re.search(expected, text), "restored suite failed")
            receipt["suite_passed"] = 6
        receipt["status"] = "RELEASE_CONTROLS_PASSED"
    except Exception as error:
        receipt["status"] = "FAILED"
        receipt["error"] = str(error)
    receipt["artifacts"] = [{"path": f.name, "sha256": hashlib.sha256(f.read_bytes()).hexdigest()}
                            for f in sorted(out.iterdir()) if f.is_file()]
    (out / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))
    return 0 if receipt["status"] == "RELEASE_CONTROLS_PASSED" else 1

if __name__ == "__main__":
    raise SystemExit(main())
