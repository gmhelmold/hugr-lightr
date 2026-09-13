#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
output="$root/benchmarks/MANIFEST.sha256"

allowlist=(
  ".github/workflows/benchmark.yml"
  "benchmarks/benchmark-spec.yaml"
  "benchmarks/RUNNER-CONTRACT.md"
  "benchmarks/fixtures/scratch-copy/Dockerfile"
  "benchmarks/fixtures/scratch-copy/data.txt"
  "benchmarks/fixtures/scratch-copy/README.md"
  "benchmarks/parity/fixture-audit.md"
  "benchmarks/parity/status-matrix.md"
  "benchmarks/parity/freeze_corpus.rb"
  "benchmarks/parity/smoke-candidates.md"
  "benchmarks/runner/Cargo.lock"
  "benchmarks/runner/Cargo.toml"
  "benchmarks/runner/src/main.rs"
  "benchmarks/reporter/__init__.py"
  "benchmarks/reporter/report.py"
  "benchmarks/reporter/tests/fixtures/raw.jsonl"
  "benchmarks/reporter/tests/test_reporter.py"
  "benchmarks/ci/docker"
  "benchmarks/ci/install_docker.sh"
  "benchmarks/ci/paths.txt"
  "benchmarks/ci/test_validate_paths.sh"
  "benchmarks/ci/validate_paths.sh"
  "benchmarks/scripts/00_verify_environment.sh"
  "benchmarks/scripts/01_clone_repos.sh"
  "benchmarks/scripts/02_extract_dockerfiles.sh"
  "benchmarks/scripts/03_convert_to_lightr.sh"
  "benchmarks/scripts/04_build_runner.sh"
  "benchmarks/scripts/05_dry_run.sh"
  "benchmarks/scripts/06_full_suite.sh"
  "benchmarks/scripts/07_analyze.sh"
  "benchmarks/scripts/08_validate.sh"
  "benchmarks/scripts/09_publish.sh"
  "benchmarks/scripts/generate_manifest.sh"
  "benchmarks/scripts/test_preparation_scripts.sh"
  "benchmarks/scripts/test_corpus.sh"
  "benchmarks/scripts/validate_contract.py"
  "benchmarks/scripts/verify_hashes.sh"
)

for path in "${allowlist[@]}"; do
  [[ -f "$root/$path" ]] || { printf 'missing manifest input: %s\n' "$path" >&2; exit 1; }
done

(
  cd "$root"
  printf '%s\n' "${allowlist[@]}" | LC_ALL=C sort | xargs sha256sum
) > "$output"
