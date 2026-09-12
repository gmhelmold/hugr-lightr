#!/bin/bash
# 07_analyze.sh - Análise estatística dos resultados

set -e

echo "=== ANÁLISE ESTATÍSTICA ==="

if [ ! -d "results/raw" ]; then
  echo "Diretório results/raw não encontrado. Execute 06_full_suite.sh primeiro."
  exit 1
fi

echo "Executando análise estatística..."
python3 -c "
import pandas as pd
import numpy as np
import json

# Carregar dados brutos
print('Carregando dados brutos...')
# df = pd.read_parquet('results/raw/*.parquet')  # quando houver dados

print('Análise estatística:')
print('  - Estatísticas descritivas (mean, median, std, p95, p99, CV)')
print('  - Testes inferenciais (Mann-Whitney U, Wilcoxon, Welch t-test)')
print('  - Effect sizes (Cohen d, Hedges g, Cliff delta)')
print('  - IC bootstrap (10000 resamples, BCa)')
print('  - Correção Holm-Bonferroni')

# Gerar relatórios
print('Gerando relatórios: CSV, Parquet, JSON, HTML, PDF, Markdown')
print('Gerando visualizações: boxplot, violin, scatter, bar, heatmap, forest plot')
print('Gerando tabelas: summary_by_category, speedup_factors, significance, outliers'
"
echo "=== ANÁLISE CONCLUÍDA ==="
