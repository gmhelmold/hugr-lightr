#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
RUNNER_DIR="$ROOT/benchmarks/runner"
: "${CARGO_BIN:?set CARGO_BIN to an absolute cargo binary path}"

case "$CARGO_BIN" in
    /*) ;;
    *) printf 'CARGO_BIN must be an absolute path: %s\n' "$CARGO_BIN" >&2; exit 1 ;;
esac
if [[ ! -x "$CARGO_BIN" ]]; then
    printf 'CARGO_BIN is not executable: %s\n' "$CARGO_BIN" >&2
    exit 1
fi
if [[ ! -f "$RUNNER_DIR/Cargo.toml" ]]; then
    printf 'bench-runner manifest not found: %s/Cargo.toml\n' "$RUNNER_DIR" >&2
    exit 1
fi

cd "$RUNNER_DIR"
"$CARGO_BIN" build --release
if [[ ! -x "$RUNNER_DIR/target/release/bench-runner" ]]; then
    printf 'bench-runner build produced no executable: %s/target/release/bench-runner\n' "$RUNNER_DIR" >&2
    exit 1
fi
