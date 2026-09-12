#!/bin/bash
# 04_build_runner.sh - Compila bench-runner

set -e

echo "=== COMPILANDO BENCH-RUNNER ==="

cd benchmarks/runner

if [ ! -f "Cargo.toml" ]; then
  echo "Cargo.toml não encontrado em benchmarks/runner/"
  exit 1
fi

echo "Compilando bench-runner (release)..."
cargo build --release

if [ -f "target/release/bench-runner" ]; then
  echo "✓ bench-runner compilado com sucesso"
  ./target/release/bench-runner --help
else
  echo "✗ Falha na compilação"
  exit 1
fi

echo "=== BENCH-RUNNER COMPILADO ==="
