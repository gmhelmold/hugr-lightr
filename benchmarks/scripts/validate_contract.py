#!/usr/bin/env python3
"""Fail-closed validation for bench-runner raw JSONL evidence."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


REQUIRED_FIELDS = frozenset(
    {
        "schema_version", "scenario_id", "availability", "tool", "round", "outcome",
        "started_at_unix_ms", "ended_at_unix_ms", "elapsed_ms", "timeout_secs",
        "exit_code", "command_sha256", "stdout_sha256", "stderr_sha256", "spec_sha256",
        "fixture_tree_sha256", "source_commit", "docker_client_version",
        "docker_server_version", "docker_api_version", "lightr_version", "lightr_sha256",
        "host_os", "host_arch", "host_kernel", "assertions",
    }
)
DIGEST_FIELDS = frozenset(
    {
        "command_sha256", "stdout_sha256", "stderr_sha256", "spec_sha256",
    }
)
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
GIT_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
AVAILABILITIES = frozenset({"supported", "unsupported", "hardware_gated", "out_of_scope"})
TOOLS = frozenset({"docker", "lightr", "skip"})
OUTCOMES = frozenset({"passed", "failed", "skipped", "timed_out"})


def jsonl_paths(input_path: Path) -> list[Path]:
    if input_path.is_file():
        return [input_path]
    if input_path.is_dir():
        return sorted(path for path in input_path.rglob("*.jsonl") if path.is_file())
    return []


def load_records(input_path: Path) -> tuple[list[dict[str, Any]], list[str]]:
    records: list[dict[str, Any]] = []
    errors: list[str] = []
    paths = jsonl_paths(input_path)
    if not paths:
        return records, [f"absent raw evidence: no JSONL under {input_path}"]

    for path in paths:
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except OSError as error:
            errors.append(f"cannot read raw evidence {path}: {error}")
            continue
        if not lines:
            errors.append(f"absent raw evidence: empty JSONL {path}")
        for line_number, line in enumerate(lines, start=1):
            if not line.strip():
                errors.append(f"malformed JSONL {path}:{line_number}: blank line")
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError as error:
                errors.append(f"malformed JSONL {path}:{line_number}: {error.msg}")
                continue
            if not isinstance(record, dict):
                errors.append(f"malformed JSONL {path}:{line_number}: record is not an object")
                continue
            records.append(record)
    return records, errors


def parse_frozen_scenarios(spec_path: Path) -> tuple[dict[str, str], list[str]]:
    """Read only frozen top-level scenario id/availability pairs; runner owns YAML parsing."""
    scenarios: dict[str, str] = {}
    errors: list[str] = []
    current_id: str | None = None
    current_availability: str | None = None

    try:
        lines = spec_path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        return scenarios, [f"cannot read spec {spec_path}: {error}"]

    def finish() -> None:
        nonlocal current_id, current_availability
        if current_id is None:
            return
        if current_availability is None:
            errors.append(f"scenario {current_id} has no availability")
        elif current_availability not in AVAILABILITIES:
            errors.append(f"scenario {current_id} has unknown availability {current_availability!r}")
        elif current_id in scenarios:
            errors.append(f"duplicate scenario id in spec: {current_id}")
        else:
            scenarios[current_id] = current_availability
        current_id = None
        current_availability = None

    for line in lines:
        scenario_match = re.match(r"^- id: ([^\s#]+)\s*$", line)
        if scenario_match:
            finish()
            current_id = scenario_match.group(1)
            continue
        availability_match = re.match(r"^  availability: ([^\s#]+)\s*$", line)
        if current_id is not None and availability_match:
            current_availability = availability_match.group(1)
    finish()
    if not scenarios and not errors:
        errors.append(f"no scenarios found in frozen spec {spec_path}")
    return scenarios, errors


def record_errors(record: dict[str, Any], location: str) -> list[str]:
    errors: list[str] = []
    missing = REQUIRED_FIELDS.difference(record)
    if missing:
        errors.append(f"{location}: missing fields {', '.join(sorted(missing))}")
        return errors
    if record["availability"] not in AVAILABILITIES:
        errors.append(f"{location}: unknown availability {record['availability']!r}")
    if record["tool"] not in TOOLS:
        errors.append(f"{location}: unknown tool {record['tool']!r}")
    if record["outcome"] not in OUTCOMES:
        errors.append(f"{location}: unknown outcome {record['outcome']!r}")
    if not isinstance(record["schema_version"], int) or isinstance(record["schema_version"], bool) or record["schema_version"] <= 0:
        errors.append(f"{location}: schema_version is not a positive integer")
    for name in DIGEST_FIELDS:
        if not isinstance(record[name], str) or not SHA256_RE.fullmatch(record[name]):
            errors.append(f"{location}: {name} is not SHA-256 hex")
    for name in ("fixture_tree_sha256", "lightr_sha256"):
        if record[name] is None and record["tool"] == "skip":
            continue
        if not isinstance(record[name], str) or not SHA256_RE.fullmatch(record[name]):
            errors.append(f"{location}: {name} is not SHA-256 hex")
    if record["source_commit"] is None and record["tool"] == "skip":
        pass
    elif not isinstance(record["source_commit"], str) or not GIT_COMMIT_RE.fullmatch(record["source_commit"]):
        errors.append(f"{location}: source_commit is not a Git commit hash")
    for name in ("round", "started_at_unix_ms", "ended_at_unix_ms", "elapsed_ms", "timeout_secs"):
        if not isinstance(record[name], int) or isinstance(record[name], bool) or record[name] < 0:
            errors.append(f"{location}: {name} is not a non-negative integer")
    if not isinstance(record["exit_code"], (int, type(None))) or isinstance(record["exit_code"], bool):
        errors.append(f"{location}: exit_code is not integer or null")
    if not isinstance(record["assertions"], list):
        errors.append(f"{location}: assertions is not a list")
    if not isinstance(record["scenario_id"], str) or not record["scenario_id"]:
        errors.append(f"{location}: scenario_id is empty")
    if record["ended_at_unix_ms"] < record["started_at_unix_ms"]:
        errors.append(f"{location}: end precedes start")
    return errors


def validate_records(records: list[dict[str, Any]], scenarios: dict[str, str], rounds: int) -> tuple[list[str], dict[str, int]]:
    errors: list[str] = []
    typed_skips = 0
    seen: set[tuple[str, str, int]] = set()
    supported_seen: set[tuple[str, str, int]] = set()

    for index, record in enumerate(records, start=1):
        location = f"record {index}"
        errors.extend(record_errors(record, location))
        if not REQUIRED_FIELDS.issubset(record):
            continue
        key = (record["scenario_id"], record["tool"], record["round"])
        if key in seen:
            errors.append(f"duplicate raw tuple: {key}")
        seen.add(key)

        scenario_availability = scenarios.get(record["scenario_id"])
        if scenario_availability is None:
            errors.append(f"{location}: scenario absent from spec: {record['scenario_id']}")
            continue
        if record["availability"] != scenario_availability:
            errors.append(f"{location}: availability differs from spec")
        if scenario_availability == "supported":
            if record["tool"] not in {"docker", "lightr"}:
                errors.append(f"{location}: supported scenario must use docker or lightr tool")
            if not isinstance(record["round"], int) or not 0 <= record["round"] < rounds:
                errors.append(f"{location}: supported round outside expected range")
            if record["outcome"] in {"failed", "timed_out"}:
                errors.append(f"{location}: supported record outcome is {record['outcome']}")
            if record["outcome"] != "passed":
                errors.append(f"{location}: supported record does not pass")
            supported_seen.add(key)
        else:
            if record["tool"] != "skip" or record["round"] != 0 or record["outcome"] != "skipped":
                errors.append(f"{location}: non-supported scenario is not typed skip")
            else:
                typed_skips += 1

    for scenario_id, availability in scenarios.items():
        if availability != "supported":
            continue
        for tool in ("docker", "lightr"):
            for round_number in range(rounds):
                expected = (scenario_id, tool, round_number)
                if expected not in supported_seen:
                    errors.append(f"missing expected supported record: {expected}")
    return errors, {"typed_skips": typed_skips, "records": len(records)}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--spec", required=True, type=Path)
    parser.add_argument("--rounds", required=True, type=int)
    args = parser.parse_args(argv)
    if args.rounds <= 0:
        parser.error("--rounds must be positive")

    records, errors = load_records(args.input)
    scenarios, spec_errors = parse_frozen_scenarios(args.spec)
    errors.extend(spec_errors)
    if not errors:
        validation_errors, summary = validate_records(records, scenarios, args.rounds)
        errors.extend(validation_errors)
    else:
        summary = {"typed_skips": 0, "records": len(records)}

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"raw evidence valid: records={summary['records']} typed_skips={summary['typed_skips']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
