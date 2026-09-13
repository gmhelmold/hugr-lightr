#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
corpus=${1:-"$root/benchmarks/s3/corpus"}
baseline=${2:?"baseline worktree required"}
ruby "$root/benchmarks/s3/corpus_gate/verify.rb" "$corpus" "$baseline"
