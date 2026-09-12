#!/bin/bash
# 00_verify_environment.sh - Verifica ambiente de execução

set -e

echo "=== VERIFICAÇÃO DE AMBIENTE ==="

# Verificar CPU idle
CPU_IDLE=$(vmstat 1 2 | tail -1 | awk '{print $15}')
echo "CPU idle: ${CPU_IDLE}%"
if [ "$CPU_IDLE" -lt 95 ]; then
  echo "⚠️  CPU idle < 95%: ${CPU_IDLE}%"
fi

# Verificar memória livre
MEM_FREE=$(vm_stat | grep 'Pages free' | awk '{print $3}' | sed 's/\.//')
echo "Memória livre: ${MEM_FREE} pages"
if [ "$MEM_FREE" -lt 2000000 ]; then
  echo "⚠️  Memória livre < 2M pages: ${MEM_FREE}"
fi

# Verificar Docker
if command -v docker &> /dev/null; then
  echo "✓ Docker: $(docker --version)"
else
  echo "✗ Docker não encontrado"
  exit 1
fi

# Verificar Lightr
if command -v lightr &> /dev/null; then
  echo "✓ Lightr: $(lightr --version 2>/dev/null || echo 'instalado')"
else
  echo "⚠️  Lightr não encontrado no PATH"
fi

# Verificar Docker daemon
if docker info &> /dev/null; then
  echo "✓ Docker daemon: rodando"
else
  echo "✗ Docker daemon não está rodando"
  exit 1
fi

# Verificar Lima VM (se no macOS)
if [[ "$OSTYPE" == "darwin"* ]]; then
  if command -v limactl &> /dev/null; then
    LIMA_STATUS=$(limactl list 2>/dev/null | grep -c "Running" || echo "0")
    if [ "$LIMA_STATUS" -gt 0 ]; then
      echo "✓ Lima VM: Running"
    else
      echo "⚠️  Lima VM: não está rodando"
    fi
  else
    echo "⚠️  limactl não encontrado"
  fi
fi

# Verificar Rust
if command -v cargo &> /dev/null; then
  echo "✓ Rust: $(cargo --version)"
else
  echo "⚠️  Rust não encontrado"
fi

# Verificar Python
if command -v python3 &> /dev/null; then
  echo "✓ Python: $(python3 --version)"
else
  echo "⚠️  Python não encontrado"
fi

echo "=== VERIFICAÇÃO DE AMBIENTE CONCLUÍDA ==="
