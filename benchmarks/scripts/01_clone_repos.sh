#!/bin/bash
# 01_clone_repos.sh - Clona repositórios nos commits fixos

set -e

echo "=== CLONANDO REPOSITÓRIOS ==="

# Kubernetes v1.29.4
if [ ! -d "kubernetes" ]; then
  echo "Clonando Kubernetes v1.29.4..."
  git clone --branch v1.29.4 --single-branch --depth 1 https://github.com/kubernetes/kubernetes.git kubernetes
else
  echo "Kubernetes já clonado"
fi

# Elasticsearch v8.13.4
if [ ! -d "elasticsearch" ]; then
  echo "Clonando Elasticsearch v8.13.4..."
  git clone --branch v8.13.4 --single-branch --depth 1 https://github.com/elastic/elasticsearch.git elasticsearch
else
  echo "Elasticsearch já clonado"
fi

# Verificar commits
echo "Verificando commits..."
cd kubernetes && git rev-parse HEAD > ../k8s_commit.txt && cd ..
cd elasticsearch && git rev-parse HEAD > ../es_commit.txt && cd ..

echo "=== REPOSITÓRIOS CLONADOS ==="
