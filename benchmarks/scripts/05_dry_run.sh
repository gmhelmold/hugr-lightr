#!/bin/bash
# 05_dry_run.sh - Dry run piloto (build-single-stage x3)

set -e

echo "=== DRY RUN PILOTO ==="

# Verificar se bench-runner existe
if [ ! -f "benchmarks/runner/target/release/bench-runner" ]; then
  echo "bench-runner não encontrado. Execute 04_build_runner.sh primeiro."
  exit 1
fi

echo "Executando dry run: build-single-stage x3 rounds..."

# Simulação do dry run (substituir por execução real do bench-runner)
for i in 1 2 3; do
  echo "Round $i/3..."
  # Simular execução
  echo "  docker build -t pause:bench -f dockerfiles/kubernetes/pause.Dockerfile ."
  echo "  lightr build -t pause:bench -f dockerfiles/kubernetes/pause.Dockerfile ."
  sleep 1
done

echo "=== DRY RUN CONCLUÍDO ==="
echo "Próximo: executar 06_full_suite.sh para suite completa"
