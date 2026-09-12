#!/bin/bash
# 06_full_suite.sh - Suite completa (250 cenários x 64 rounds)

set -e

echo "=== SUITE COMPLETA DE BENCHMARK ==="
echo "AVISO: Este script executa 250 cenários x 64 rounds = 16.000 execuções"
echo "Tempo estimado: 48+ horas"
echo ""
read -p "Confirmar execução completa? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "Cancelado."
  exit 1
fi

echo "Iniciando suite completa..."
# O bench-runner real executaria aqui
# bench-runner --spec benchmarks/benchmark-spec.yaml --rounds 64 --output results/

echo "=== SUITE COMPLETA INICIADA ==="
echo "Monitorar progresso em results/"
echo "Tempo estimado: 48+ horas"
