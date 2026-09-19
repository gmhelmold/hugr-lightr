"""Bound the complete native CRI test binary; a timeout is a retained failure."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
ORIGINAL = "stream::tests::open_exec_tty_uses_pty_master_no_stderr"
REGRESSION = "stream::tty_setup::tests::delayed_reader_keeps_output_after_all_child_stdio_close"
DESCRIPTOR_TESTS = ["stream_io::descriptor_tests::" + name for name in (
    "pty_descriptors_are_close_on_exec_at_return",
    "pty_descriptor_duplicates_are_close_on_exec",
    "pty_descriptors_are_absent_after_unrelated_exec")]


def error_record(error):
    """Serializable native/process error; errno and timeout are not inferred."""
    record = {"type": type(error).__name__, "message": str(error)}
    for name in ("errno", "timeout"):
        value = getattr(error, name, None)
        if value is not None:
            record[name] = value
    return record


def terminate_and_reap(child, record):
    """One termination request and a bounded wait, not a cleanup guarantee."""
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass  # It may have exited between the initial timeout and the signal.
    except OSError as error:
        record["errors"]["terminate"] = error_record(error)
    try:
        record["exit"] = child.wait(timeout=30)
        record["reaped"] = True
    except (subprocess.TimeoutExpired, OSError) as error:
        # In particular, a second timeout must not erase the first failure.
        # A child blocked in native code may remain unreaped. Report that fact;
        # do not invent an exit code or retry until a green outcome appears.
        record["errors"]["reap"] = error_record(error)


def execute(command, root, env, log, limit):
    """Retain spawn/wait/termination outcomes, including incomplete cleanup."""
    started = time.monotonic()
    record = {"command": command, "exit": None, "timed_out": False,
              "reaped": False, "errors": {}, "limit_seconds": limit, "log": log.name}
    with log.open("wb") as stream:
        try:
            child = subprocess.Popen(command, cwd=root, env=env, stdout=stream,
                                     stderr=subprocess.STDOUT, start_new_session=True)
        except OSError as error:
            record["errors"]["spawn"] = error_record(error)
        else:
            try:
                record["exit"] = child.wait(timeout=limit)
                record["reaped"] = True
            except subprocess.TimeoutExpired:
                record["timed_out"] = True
                terminate_and_reap(child, record)
            except OSError as error:
                record["errors"]["wait"] = error_record(error)
                terminate_and_reap(child, record)
    # This is a point-in-time byte snapshot. An unreaped child or an escaped
    # descendant can still hold the original log fd; do not claim a final log
    # or process-tree cleanup from the direct child's reaped state alone.
    data = log.read_bytes()
    record.update(seconds=round(time.monotonic() - started, 3),
                  sha256=hashlib.sha256(data).hexdigest(),
                  log_snapshot_bytes=len(data), log_hash_scope="snapshot_at_return")
    return record


def test_progress(text, names):
    """Diagnostic inventory only. Missing completion does not prove a hang."""
    results = re.findall(r"^test ([^\r\n]+) \.\.\. (ok|FAILED|ignored)$", text, re.M)
    observed = {name for name, _ in results}
    long_running = set(re.findall(
        r"^test ([^\r\n]+) has been running for over [0-9]+ seconds$", text, re.M))
    return {
        "passed": [name for name, status in results if status == "ok"],
        "failed": [name for name, status in results if status == "FAILED"],
        "ignored": [name for name, status in results if status == "ignored"],
        "without_result": [name for name in names if name not in observed],
        "long_running": [name for name in names if name in long_running],
    }


def listed_tests(text):
    names = re.findall(r"^([^\r\n]+): test$", text, re.M)
    if len(names) != len(set(names)) or not {ORIGINAL, REGRESSION, *DESCRIPTOR_TESTS} <= set(names):
        raise ValueError("required native methods missing or duplicate in test inventory")
    return names


def validate_full_suite(record, text, names):
    if record["timed_out"] or record["exit"] != 0 or record.get("errors"):
        raise ValueError("full native CRI suite failed or timed out")
    observed = re.findall(r"^test ([^\r\n]+) \.\.\. ok$", text, re.M)
    if sorted(observed) != sorted(names):
        raise ValueError("full suite did not pass every listed method exactly once")
    expected = (f"test result: ok. {len(names)} passed; 0 failed; 0 ignored; "
                "0 measured; 0 filtered out;")
    if text.count(expected) != 1:
        raise ValueError("missing full-suite summary or nonzero skipped/filtered count")


def select_executable(text, root):
    candidates = []
    for line in text.splitlines():
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if (item.get("reason") == "compiler-artifact"
                and item.get("target", {}).get("name") == "lightr_cri_backend"
                and item.get("profile", {}).get("test") is True
                and item.get("executable")):
            path = Path(item["executable"]).resolve()
            if not path.is_relative_to((root / "target").resolve()) or not path.is_file():
                raise ValueError("compiled test executable is outside the owned target directory")
            candidates.append(path)
    if len(candidates) != 1:
        raise ValueError("expected exactly one native CRI test executable")
    return candidates[0]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    out = parser.parse_args().out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    receipt = {"status": "FAILED", "commands": [], "platform": platform.platform(),
               "machine": platform.machine(), "production_protocol_enabled": False}
    env = dict(os.environ, CARGO_INCREMENTAL="0", CARGO_TERM_COLOR="never",
               CARGO_BUILD_JOBS="2", CARGO_TARGET_DIR=str(ROOT / "target"),
               RUSTFLAGS="-D warnings -C link-arg=-Wl,-rpath,/usr/lib/swift")

    def run(command, label, limit):
        record = execute(command, ROOT, env, out / (label + ".log"), limit)
        receipt["commands"].append(dict(label=label, **record))
        if label == "full-cri-suite":
            receipt["test_progress"] = test_progress(
                (out / record["log"]).read_text(errors="replace"),
                receipt.get("required_methods", []))
        (out / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
        if record["timed_out"] or record["exit"] != 0 or record.get("errors"):
            raise RuntimeError("native command failed or timed out: " + label)
        return record, (out / record["log"]).read_text(errors="replace")

    try:
        if platform.system() != "Darwin":
            raise RuntimeError("native Darwin required; no capability skip")
        for name, arguments in [("checkout", ["HEAD"]), ("tree", ["HEAD^{tree}"])]:
            receipt[name] = subprocess.check_output(
                ["git", "rev-parse", *arguments], cwd=ROOT, text=True, timeout=10).strip()
        receipt["compiler"] = subprocess.check_output(
            ["rustc", "+1.96.0", "-Vv"], cwd=ROOT, text=True, timeout=30)
        _, text = run(["cargo", "+1.96.0", "test", "--locked", "-p", "lightr-cri-backend",
                       "--features", "lightr-run/vz", "--lib", "--no-run",
                       "--message-format=json"], "build", 900)
        binary = select_executable(text, ROOT)
        receipt["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
        _, text = run([str(binary), "--list"], "inventory", 30)
        names = listed_tests(text)
        receipt["required_methods"] = names
        record, text = run([str(binary), "--test-threads=4"], "full-cri-suite", 90)
        validate_full_suite(record, text, names)
        receipt["status"] = "NATIVE_CRI_FULL_SUITE_PASSED"
    except Exception as error:
        receipt["error"] = str(error)
    (out / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2), flush=True)
    return 0 if receipt["status"] == "NATIVE_CRI_FULL_SUITE_PASSED" else 1


if __name__ == "__main__":
    raise SystemExit(main())
