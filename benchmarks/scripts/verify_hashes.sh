#!/bin/bash
# verify_hashes.sh - Verifica commit hashes dos repositórios

set -e

echo "=== VERIFICAÇÃO DE COMMIT HASHES ==="
echo ""

# Kubernetes
echo "Verificando Kubernetes v1.29.4..."
K8S_HASH=$(git ls-remote https://github.com/kubernetes/kubernetes.git v1.29.4 | cut -f1)
K8S_EXPECTED="5d985937e8a112a321916efd4ad3936c7db6345f"

echo "Kubernetes v1.29.4:"
echo "  Esperado: $K8S_EXPECTED"
echo "  Atual:    $(git ls-remote https://github.com/kubernetes/kubernetes.git v1.29.4 | cut -f1)"

if [ "$(git ls-remote https://github.com/kubernetes/kubernetes.git v1.29.4 | cut -f1)" = "5d985937e8a112a321916efd4ad3936c7db6345f" ]; then
  echo "✓ Kubernetes v1.29.4: OK"
else
  echo "✗ Kubernetes hash MISMATCH!"
  exit 1
fi

echo ""

# Elasticsearch
echo "Verificando Elasticsearch v8.13.4..."
ES_EXPECTED="85ff3fe65dcf2ab0185083aee4b8f462a92ab289"

echo "Elasticsearch v8.13.4:"
echo "  Esperado: $ES_EXPECTED"
echo "  Atual:    $(git ls-remote https://github.com/elastic/elasticsearch.git v8.13.4 | cut -f1)"

if [ "$(git ls-remote https://github.com/elastic/elasticsearch.git v8.13.4 | cut -f1)" = "85ff3fe65dcf2ab0185083aee4b8f462a92ab289" ]; then
  echo "✓ Elasticsearch v8.13.4: OK"
else
  echo "✗ Elasticsearch hash MISMATCH!"
  exit 1
fi

echo ""
echo "=== TODOS OS HASHES CONFIRMADOS ==="
