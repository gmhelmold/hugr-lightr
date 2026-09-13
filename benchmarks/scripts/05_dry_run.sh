#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
SPEC="$ROOT/benchmarks/benchmark-spec.yaml"
RUNNER_BIN="$ROOT/benchmarks/runner/target/release/bench-runner"
LIGHTR_BIN="$ROOT/target/release/lightr"
OUT_DIR="$ROOT/benchmarks/work/results/dry-run"
: "${DOCKER_BIN:?set DOCKER_BIN to an absolute Docker binary path}"
: "${CHUNK:?set CHUNK to selected chunk index}"
: "${TOTAL_CHUNKS:?set TOTAL_CHUNKS to total chunk count}"

require_absolute_executable() {
    local path=$1
    local name=$2
    case "$path" in
        /*) ;;
        *) printf '%s must be an absolute path: %s\n' "$name" "$path" >&2; exit 1 ;;
    esac
    if [[ ! -x "$path" ]]; then
        printf '%s is not executable: %s\n' "$name" "$path" >&2
        exit 1
    fi
}

if [[ ! "$CHUNK" =~ ^[0-9]+$ || ! "$TOTAL_CHUNKS" =~ ^[1-9][0-9]*$ || "$CHUNK" -ge "$TOTAL_CHUNKS" ]]; then
    printf 'invalid chunk selection: CHUNK=%s TOTAL_CHUNKS=%s\n' "$CHUNK" "$TOTAL_CHUNKS" >&2
    exit 1
fi
require_absolute_executable "$RUNNER_BIN" RUNNER_BIN
require_absolute_executable "$DOCKER_BIN" DOCKER_BIN
require_absolute_executable "$LIGHTR_BIN" LIGHTR_BIN
bash "$ROOT/benchmarks/scripts/00_verify_environment.sh"
mkdir -p "$OUT_DIR/chunk-$CHUNK"

"$RUNNER_BIN" verify-spec --spec "$SPEC"
"$RUNNER_BIN" run --spec "$SPEC" --chunk "$CHUNK" --chunks "$TOTAL_CHUNKS" --rounds 3 \
    --out "$OUT_DIR/chunk-$CHUNK" --docker "$DOCKER_BIN" --lightr "$LIGHTR_BIN"
