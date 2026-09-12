#!/usr/bin/env bash
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
MERGED_JSONL="${MERGED_JSONL:-$ROOT/results/merged/merged.jsonl}"
: "${ROUNDS:?ROUNDS must match bench-runner --rounds}"

if [ ! -f "$MERGED_JSONL" ]; then
    printf 'merged raw evidence not found: %s\n' "$MERGED_JSONL" >&2
    exit 1
fi

python3 "$ROOT/benchmarks/scripts/validate_contract.py" \
    --input "$MERGED_JSONL" \
    --spec "$ROOT/benchmarks/benchmark-spec.yaml" \
    --rounds "$ROUNDS"
