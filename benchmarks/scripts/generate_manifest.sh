#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
output="$root/benchmarks/MANIFEST.sha256"

allowlist=(
  "benchmarks/benchmark-spec.yaml"
  "benchmarks/RUNNER-CONTRACT.md"
  "benchmarks/fixtures/scratch-copy/Dockerfile"
  "benchmarks/fixtures/scratch-copy/data.txt"
  "benchmarks/fixtures/scratch-copy/README.md"
  "benchmarks/parity/fixture-audit.md"
  "benchmarks/parity/status-matrix.md"
  "benchmarks/parity/freeze_corpus.rb"
  "benchmarks/scripts/generate_manifest.sh"
  "benchmarks/scripts/test_corpus.sh"
)

for path in "${allowlist[@]}"; do
  [[ -f "$root/$path" ]] || { printf 'missing manifest input: %s\n' "$path" >&2; exit 1; }
done

(
  cd "$root"
  printf '%s\n' "${allowlist[@]}" | LC_ALL=C sort | xargs sha256sum
) > "$output"
