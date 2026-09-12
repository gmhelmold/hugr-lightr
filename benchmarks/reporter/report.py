#!/usr/bin/env python3
"""Emit count-only CSV, JSON, and Markdown reports from raw JSONL evidence."""

from __future__ import annotations

import argparse
import csv
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any


NOT_COMPUTED = "not_computed"
STAT_FIELDS = ("mean_elapsed_ms", "median_elapsed_ms", "p95_elapsed_ms", "stddev_elapsed_ms")


def jsonl_paths(input_path: Path) -> list[Path]:
    if input_path.is_file():
        return [input_path]
    if input_path.is_dir():
        return sorted(path for path in input_path.rglob("*.jsonl") if path.is_file())
    return []


def load_records(input_path: Path) -> list[dict[str, Any]]:
    paths = jsonl_paths(input_path)
    if not paths:
        raise ValueError(f"absent raw evidence: no JSONL under {input_path}")
    records: list[dict[str, Any]] = []
    for path in paths:
        lines = path.read_text(encoding="utf-8").splitlines()
        if not lines:
            raise ValueError(f"absent raw evidence: empty JSONL {path}")
        for line_number, line in enumerate(lines, start=1):
            if not line.strip():
                raise ValueError(f"malformed JSONL {path}:{line_number}: blank line")
            try:
                record = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"malformed JSONL {path}:{line_number}: {error.msg}") from error
            if not isinstance(record, dict):
                raise ValueError(f"malformed JSONL {path}:{line_number}: record is not an object")
            records.append(record)
    return records


def report_rows(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    grouped: dict[tuple[str, str, str], Counter[str]] = {}
    for record in records:
        try:
            key = (record["scenario_id"], record["availability"], record["tool"])
            outcome = record["outcome"]
        except KeyError as error:
            raise ValueError(f"raw record missing report field: {error.args[0]}") from error
        if not all(isinstance(value, str) and value for value in key) or not isinstance(outcome, str):
            raise ValueError("raw record has invalid report fields")
        grouped.setdefault(key, Counter())[outcome] += 1

    rows: list[dict[str, Any]] = []
    for (scenario_id, availability, tool), outcomes in sorted(grouped.items()):
        row: dict[str, Any] = {
            "scenario_id": scenario_id,
            "availability": availability,
            "tool": tool,
            "records": sum(outcomes.values()),
            "passed": outcomes["passed"],
            "failed": outcomes["failed"],
            "timed_out": outcomes["timed_out"],
            "skipped": outcomes["skipped"],
        }
        row.update({field: NOT_COMPUTED for field in STAT_FIELDS})
        rows.append(row)
    return rows


def write_reports(rows: list[dict[str, Any]], output_dir: Path) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    fields = list(rows[0]) if rows else [
        "scenario_id", "availability", "tool", "records", "passed", "failed", "timed_out", "skipped", *STAT_FIELDS,
    ]
    with (output_dir / "summary.csv").open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)

    payload = {"schema_version": 1, "statistics": NOT_COMPUTED, "rows": rows}
    (output_dir / "summary.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    markdown = ["# Raw Evidence Summary", "", "Statistics: `not_computed`.", ""]
    markdown.append("| " + " | ".join(fields) + " |")
    markdown.append("| " + " | ".join("---" for _ in fields) + " |")
    for row in rows:
        markdown.append("| " + " | ".join(str(row[field]) for field in fields) + " |")
    (output_dir / "summary.md").write_text("\n".join(markdown) + "\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        rows = report_rows(load_records(args.input))
        write_reports(rows, args.out)
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"reports written: {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
