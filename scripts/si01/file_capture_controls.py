"""Compile cause-specific capture defects in disposable source; never mutate live data."""
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
BASE = "crates/lightr-store/src/store/cas/"
PREFIX = "store::cas::preparation::file_capture::"
CASES = [
    ("verify", "preparation.rs",
     "if length != copied || expected.is_some_and(|value| value != (digest, length)) {",
     "if false && (length != copied || expected.is_some_and(|value| value != (digest, length))) {",
     "file_capture_claimed_clone_success_still_checks_digest_and_length",
     "claimed clone skipped captured-byte verification"),
    ("resource", "file_capture.rs", "Err(error) if native::can_fallback(&error) => {",
     "Err(error) if native::can_fallback(&error) || true => {",
     "file_capture_clone_resource_error_is_not_hidden_by_fallback",
     "resource failure fell through to copy"),
    ("sync", "preparation.rs", "sync(&file).map_err(|error| failure(Phase::PayloadSync, error))?;",
     "let _ = &sync;", "file_capture_sync_error_remains_fatal_for_both_capture_modes",
     "capture ignored file synchronization failure"),
    ("hash-checkpoint", "preparation.rs", "Digest::of_reader_checked(&mut file, checkpoint)",
     "Digest::of_reader(&mut file)",
     "file_capture_cancel_after_clone_and_during_hash_cleans_only_owned_file", "right: Verify"),
]


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
    expected = r"test result: ok\. 13 passed; 0 failed; 0 ignored;"
    try:
        require(not git("status", "--porcelain", "--untracked-files=no"), "tracked source dirty")
        archive = subprocess.check_output(["git", "archive", "--format=zip", "HEAD"], cwd=ROOT)
        (out / "source.zip").write_bytes(archive)
        with tempfile.TemporaryDirectory(prefix="si01-file-capture-controls-") as temporary:
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
                    full = PREFIX + "tests::" + test
                    code, text = run(root, cargo + [full, "--", "--exact"], label + "-test", 60)
                    require(code == 101 and f"test {full} ... FAILED" in text and message in text
                            and re.search(r"test result: FAILED\. 0 passed; 1 failed; 0 ignored;", text),
                            "expected causal assertion absent: " + label)
                    receipt["controls"].append(dict(name=label, build_exit=0, test_exit=101, test=full))
                finally:
                    path.write_text(original)
            code, text = run(root, suite, "restored")
            require(code == 0 and re.search(expected, text), "restored suite failed")
        receipt["status"] = "CAPTURE_CONTROLS_PASSED"
    except Exception as error:
        receipt["status"] = "FAILED"
        receipt["error"] = str(error)
    receipt["artifacts"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(out.iterdir()) if p.is_file()}
    (out / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))
    return 0 if receipt["status"] == "CAPTURE_CONTROLS_PASSED" else 1


if __name__ == "__main__":
    raise SystemExit(main())
