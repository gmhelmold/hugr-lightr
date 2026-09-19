"""Native Darwin PTY regression and compiled control in disposable source."""
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
PREFIX = "stream::tty_setup::tests::"
TESTS = ["child_closes_standard_streams", "delayed_reader_keeps_output_after_all_child_stdio_close",
         "closing_the_master_releases_the_exiting_child", "empty_output_reaps_without_a_reader",
         "setup_error_does_not_spawn_a_non_tty_workload", "actual_exec_keeps_raw_master_merged_output_and_exit_code", "invalid_slave_descriptor_preserves_ebadf"]
ORIGINAL = "stream::tests::open_exec_tty_uses_pty_master_no_stderr"
SOURCE = "crates/lightr-cri-backend/src/stream_tty.rs"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    out = parser.parse_args().out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    git = lambda *args: subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()
    receipt = {"head": git("rev-parse", "HEAD"), "tree": git("rev-parse", "HEAD^{tree}"),
               "commands": [], "production_protocol_enabled": False}

    def require(condition, message):
        if not condition:
            raise RuntimeError(message)

    def run(root, command, label, limit=900):
        log = out / (label + ".log")
        env = os.environ.copy()
        env.update(CARGO_BUILD_JOBS="2", CARGO_INCREMENTAL="0", CARGO_PROFILE_TEST_DEBUG="0",
                   CARGO_PROFILE_DEV_DEBUG="0", CARGO_TERM_COLOR="never", CARGO_TARGET_DIR=str(root / "target"),
                   RUSTFLAGS="-D warnings -C link-arg=-Wl,-rpath,/usr/lib/swift")
        with log.open("wb") as stream:
            child = subprocess.Popen(command, cwd=root, env=env, stdout=stream,
                                     stderr=subprocess.STDOUT, start_new_session=True)
            try:
                code = child.wait(timeout=limit)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=30)
                raise RuntimeError("watchdog is not a passing control: " + label)
        receipt["commands"].append({"label": label, "command": command, "exit": code})
        return code, log.read_text(errors="replace")

    cargo = ["cargo", "+1.96.0", "test", "--locked", "-p", "lightr-cri-backend", "--lib"]

    def suites(root, label):
        code, text = run(root, cargo + [PREFIX, "--", "--test-threads=4"], label + "-regressions")
        require(code == 0 and re.search(r"test result: ok\. 7 passed; 0 failed; 0 ignored;", text),
                "seven-method native suite failed or missing: " + label)
        for name in TESTS:
            require("test " + PREFIX + name + " ... ok" in text, "required witness absent: " + name)
        code, text = run(root, cargo + [ORIGINAL, "--", "--exact"], label + "-original")
        require(code == 0 and "test " + ORIGINAL + " ... ok" in text
                and re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored;", text),
                "original unmodified echo witness failed: " + label)

    try:
        require(os.uname().sysname == "Darwin", "native Darwin required, never a skipped capability pass")
        require(not git("status", "--porcelain", "--untracked-files=no"), "tracked source dirty")
        receipt["host"] = subprocess.check_output(["uname", "-ms"], text=True).strip()
        receipt["os"] = subprocess.check_output(["sw_vers"], text=True)
        receipt["compiler"] = subprocess.check_output(["rustc", "+1.96.0", "-Vv"], text=True)
        archive = subprocess.check_output(["git", "archive", "--format=zip", "HEAD"], cwd=ROOT)
        (out / "source.zip").write_bytes(archive)
        with tempfile.TemporaryDirectory(prefix="lightr-pty-control-") as scratch:
            root = Path(scratch).resolve()
            with zipfile.ZipFile(io.BytesIO(archive)) as package:
                for member in package.infolist():
                    require((root / member.filename).resolve().is_relative_to(root), "archive path outside scratch")
                package.extractall(root)
            suites(root, "pristine")
            source = root / SOURCE
            original = source.read_text()
            old = 'c"/dev/tty".as_ptr()'
            require(original.count(old) == 1, "controlling-terminal seam changed")
            source.write_text(original.replace(old, 'c"/dev/null".as_ptr()', 1))
            try:
                code, _ = run(root, cargo + ["--no-run"], "missing-reference-build")
                require(code == 0, "control must compile")
                name = PREFIX + "delayed_reader_keeps_output_after_all_child_stdio_close"
                code, text = run(root, cargo + [name, "--", "--exact"], "missing-reference-test", 60)
                require(code == 101 and "test " + name + " ... FAILED" in text
                        and "queued PTY output was lost" in text
                        and re.search(r"test result: FAILED\. 0 passed; 1 failed; 0 ignored;", text),
                        "missing-reference control did not fail its causal assertion")
                receipt["control"] = {"build_exit": 0, "test_exit": 101, "name": name}
            finally:
                source.write_text(original)
            suites(root, "restored")
        receipt["status"] = "NATIVE_PTY_CONTROLS_PASSED"
    except Exception as error:
        receipt.update(status="FAILED", error=str(error))
    receipt["artifacts"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                            for p in sorted(out.iterdir()) if p.is_file()}
    (out / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))
    return 0 if receipt["status"] == "NATIVE_PTY_CONTROLS_PASSED" else 1


if __name__ == "__main__":
    raise SystemExit(main())
