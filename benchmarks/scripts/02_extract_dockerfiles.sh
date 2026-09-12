#!/bin/bash
# 02_extract_dockerfiles.sh - Extrai Dockerfiles e compose files

set -e

echo "=== EXTRAINDO DOCKERFILES ==="

mkdir -p dockerfiles/kubernetes
mkdir -p dockerfiles/elasticsearch
mkdir -p composefiles/kubernetes
mkdir -p composefiles/elasticsearch

# Kubernetes Dockerfiles
echo "Copiando Dockerfiles do Kubernetes..."
cp kubernetes/build/pause/Dockerfile dockerfiles/kubernetes/pause.Dockerfile 2>/dev/null || echo "pause.Dockerfile não encontrado"
cp kubernetes/build/debian-base/Dockerfile dockerfiles/kubernetes/debian-base.Dockerfile 2>/dev/null || echo "debian-base.Dockerfile não encontrado"
cp kubernetes/build/kube-apiserver/Dockerfile dockerfiles/kubernetes/kube-apiserver.Dockerfile 2>/dev/null || echo "kube-apiserver.Dockerfile não encontrado"
cp kubernetes/build/kube-controller-manager/Dockerfile dockerfiles/kubernetes/kube-controller-manager.Dockerfile 2>/dev/null || echo "kube-controller-manager.Dockerfile não encontrado"
cp kubernetes/build/kube-scheduler/Dockerfile dockerfiles/kubernetes/kube-scheduler.Dockerfile 2>/dev/null || echo "kube-scheduler.Dockerfile não encontrado"
cp kubernetes/build/kube-proxy/Dockerfile dockerfiles/kubernetes/kube-proxy.Dockerfile 2>/dev/null || echo "kube-proxy.Dockerfile não encontrado"
cp kubernetes/build/kubectl/Dockerfile dockerfiles/kubernetes/kubectl.Dockerfile 2>/dev/null || echo "kubectl.Dockerfile não encontrado"
cp kubernetes/cluster/images/etcd/Dockerfile dockerfiles/kubernetes/etcd.Dockerfile 2>/dev/null || echo "etcd.Dockerfile não encontrado"
cp kubernetes/cluster/images/coredns/Dockerfile dockerfiles/kubernetes/coredns.Dockerfile 2>/dev/null || echo "coredns.Dockerfile não encontrado"
cp kubernetes/cmd/kind/Dockerfile dockerfiles/kubernetes/kind-node.Dockerfile 2>/dev/null || echo "kind-node.Dockerfile não encontrado"

# Elasticsearch Dockerfiles
cp elasticsearch/docker/Dockerfile dockerfiles/elasticsearch/elasticsearch.Dockerfile 2>/dev/null || echo "elasticsearch.Dockerfile não encontrado"
cp elasticsearch/docker/Dockerfile.arm64 dockerfiles/elasticsearch/elasticsearch-arm64.Dockerfile 2>/dev/null || echo "elasticsearch-arm64.Dockerfile não encontrado"
cp elasticsearch/docker/Dockerfile.fips dockerfiles/elasticsearch/elasticsearch-fips.Dockerfile 2>/dev/null || echo "elasticsearch-fips.Dockerfile não encontrado"

# Compose files Kubernetes
cp kubernetes/kind/examples/multi-node.yaml composefiles/kubernetes/multi-node.yaml 2>/dev/null || echo "multi-node.yaml não encontrado"
cp kubernetes/kind/examples/kind-with-registry.yaml composefiles/kubernetes/kind-with-registry.yaml 2>/dev/null || echo "kind-with-registry.yaml não encontrado"
cp kubernetes/kind/examples/kind-with-calico.yaml composefiles/kubernetes/kind-with-calico.yaml 2>/dev/null || echo "kind-with-calico.yaml não encontrado"
cp kubernetes/kind/examples/kind-with-cilium.yaml composefiles/kubernetes/kind-with-cilium.yaml 2>/dev/null || echo "kind-with-cilium.yaml não encontrado"

# Compose files Elasticsearch
cp elasticsearch/docker-compose.yml composefiles/elasticsearch/single.yml 2>/dev/null || echo "single.yml não encontrado"
cp elasticsearch/docker-compose.cluster.yml composefiles/elasticsearch/cluster.yml 2>/dev/null || echo "cluster.yml não encontrado"
cp elasticsearch/docker-compose.security.yml composefiles/elasticsearch/security.yml 2>/dev/null || echo "security.yml não encontrado"
cp elasticsearch/docker-compose.snapshot.yml composefiles/elasticsearch/snapshot.yml 2>/dev/null || echo "snapshot.yml não encontrado"
cp elasticsearch/docker-compose.ccr.yml composefiles/elasticsearch/ccr.yml 2>/dev/null || echo "ccr.yml não encontrado"
cp elasticsearch/docker-compose.ilm.yml composefiles/elasticsearch/ilm.yml 2>/dev/null || echo "ilm.yml não encontrado"
cp elasticsearch/docker-compose.beats.yml composefiles/elasticsearch/beats.yml 2>/dev/null || echo "beats.yml não encontrado"
cp elasticsearch/docker-compose.fleet.yml composefiles/elasticsearch/fleet.yml 2>/dev/null || echo "fleet.yml não encontrado"

echo "=== DOCKERFILES EXTRAÍDOS ==="
