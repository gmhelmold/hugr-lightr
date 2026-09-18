"""Fail closed unless every mandatory job in the shared CI succeeded.

GitHub passes only job results via CI_NEEDS_JSON, never a shell command.
This aggregates executed gates; it does not certify runtime completeness.
"""
import json
import os
import sys

REQUIRED_JOBS = frozenset({
    "linux-x86_64", "linux-aarch64", "macos-x86_64", "macos-arm64",
    "windows", "cross-compile", "macos-current-arm64", "docs-spec", "benchmark-report",
})


def validate(needs):
    if not isinstance(needs, dict) or set(needs) != REQUIRED_JOBS:
        raise ValueError("required CI job set missing or changed")
    failed = [name for name, job in sorted(needs.items())
              if not isinstance(job, dict) or job.get("result") != "success"]
    if failed:
        raise ValueError("mandatory jobs did not succeed: " + ", ".join(failed))


def main():
    try:
        validate(json.loads(os.environ["CI_NEEDS_JSON"]))
    except (KeyError, TypeError, ValueError) as error:
        print("Required CI: FAIL: " + str(error), file=sys.stderr)
        return 1
    print("Required CI: all nine mandatory jobs succeeded; release excluded")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
