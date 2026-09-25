"""Repeat the exact existing feature-enabled socket witness; first failure blocks.

Do not retry to green. Build once, record the executable, then use Cargo to execute exactly
one enumerated case per iteration (preserving its dynamic-library environment). Neither EINVAL nor a timeout is suppressed.
"""
import hashlib
import json
import os
import platform
import re
import signal
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "socket-witness-evidence"
CASE = "vswitch::switch_host::tests::attach_forward_dhcp_dns_then_refcount_self_stop"


def exact_pass(code, text):
    return (code == 0
            and re.search(r"^test " + re.escape(CASE) + r" \.\.\. ok$", text, re.M) is not None
            and re.search(r"^test result: ok\. 1 passed; 0 failed; 0 ignored;", text, re.M) is not None)


def invoke(argv, log, timeout):
    with log.open("wb") as output:
        process = subprocess.Popen(argv, cwd=ROOT, stdout=output,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=30)
            raise RuntimeError("socket witness watchdog expired; not a pass or retry")
    return code, log.read_text(errors="replace")


def main():
    OUT.mkdir(exist_ok=False)
    receipt = {"checkout": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
               "tree": subprocess.check_output(["git", "rev-parse", "HEAD^{tree}"], cwd=ROOT, text=True).strip(),
               "machine": platform.machine(), "platform": platform.platform(),
               "case": CASE, "features": ["vz"], "rustflags": os.environ.get("RUSTFLAGS", ""),
               "iterations": [], "status": "FAILED"}
    try:
        for args in (["git", "diff", "--exit-code"], ["git", "diff", "--cached", "--exit-code"]):
            subprocess.run(args, cwd=ROOT, check=True, capture_output=True)
        code, text = invoke(["cargo", "+1.96.0", "test", "--locked", "-p", "lightr-run",
                             "--lib", "--features", "vz", "--no-run", "--message-format=json"],
                            OUT / "build.log", 600)
        if code != 0:
            raise RuntimeError("socket witness build failed")
        binaries = set()
        for line in text.splitlines():
            try:
                message = json.loads(line)
            except ValueError:
                continue
            if (message.get("reason") == "compiler-artifact" and message.get("executable")
                    and message.get("profile", {}).get("test") is True
                    and message.get("target", {}).get("name") == "lightr_run"):
                binaries.add(message["executable"])
        if len(binaries) != 1:
            raise RuntimeError("expected one lightr-run test executable")
        binary = Path(binaries.pop()).resolve()
        receipt["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
        cargo_case = ["cargo", "+1.96.0", "test", "--locked", "-p", "lightr-run",
                      "--lib", "--features", "vz", CASE, "--", "--exact"]
        code, text = invoke(cargo_case + ["--list"], OUT / "list.log", 60)
        if code != 0 or CASE + ": test" not in text or "1 test, 0 benchmarks" not in text:
            raise RuntimeError("exact socket case enumeration failed; inspect list.log")
        for index in range(10):
            code, text = invoke(cargo_case + ["--nocapture"], OUT / (str(index) + ".log"), 60)
            ok = exact_pass(code, text)
            receipt["iterations"].append({"index": index, "exit": code, "passed": ok})
            if not ok:
                print(text[-8000:])
                raise RuntimeError("exact socket witness failed at iteration " + str(index))
        for args in (["git", "diff", "--exit-code"], ["git", "diff", "--cached", "--exit-code"]):
            subprocess.run(args, cwd=ROOT, check=True, capture_output=True)
        if hashlib.sha256(binary.read_bytes()).hexdigest() != receipt["binary_sha256"]:
            raise RuntimeError("socket witness binary changed during execution")
        receipt["status"] = "PASSED"
    except Exception as error:
        receipt["error"] = str(error)
    finally:
        receipt["logs"] = [{"path": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                           for path in sorted(OUT.glob("*.log"))]
        (OUT / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))
    return 0 if receipt["status"] == "PASSED" else 1


if __name__ == "__main__":
    raise SystemExit(main())
