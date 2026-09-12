#!/bin/bash
# 03_convert_to_lightr.sh - Conversão manual Dockerfile -> Lightr spec

set -e

echo "=== CONVERSÃO DOCKERFILE -> LIGHTR SPEC ==="

echo "NOTA: Esta etapa requer revisão manual."
echo "Os Dockerfiles foram extraídos para dockerfiles/"
echo "Os compose files foram extraídos para composefiles/"
echo ""
echo "Para cada Dockerfile, crie o Lightr spec equivalente:"
echo "  - FROM -> base.image"
echo "  - RUN -> exec.commands"
echo "  - COPY/ADD -> files.copy"
echo "  - ENV -> env.vars"
echo "  - EXPOSE -> network.expose"
echo "  - VOLUME -> volumes.mounts"
echo "  - USER -> security.user"
echo "  - WORKDIR -> working_dir"
echo "  - ENTRYPOINT/CMD -> entrypoint / command"
echo ""
echo "Salve os specs em lightr-specs/"
echo ""
echo "Revisão manual necessária antes de prosseguir."
echo "Pressione Enter para continuar após revisão..."
read -p ""

echo "=== CONVERSÃO CONCLUÍDA (REVISÃO MANUAL) ==="
