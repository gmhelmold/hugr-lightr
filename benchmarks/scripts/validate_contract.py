#!/usr/bin/env python3
# validate_contract.py - Valida contrato WAVE-EXP

import yaml
import sys
import re

def validate_contract():
    print("=== VALIDAÇÃO DO CONTRATO WAVE-EXP ===")
    print()
    
    with open('benchmarks/benchmark-spec.yaml') as f:
        data = yaml.safe_load(f)
    
    scenarios = data.get('scenarios', [])
    errors = []
    warnings = []
    
    # 1. Verificar IDs únicos
    ids = [s.get('id') for s in scenarios if s.get('id')]
    duplicates = [id for id in set(ids) if ids.count(id) > 1]
    if duplicates:
        errors.append(f"IDs duplicados: {duplicates}")
    else:
        print("✓ IDs únicos: OK")
    
    # 2. Verificar campos obrigatórios
    required_fields = ['category', 'id', 'project', 'docker_cmd', 'lightr_cmd', 'metrics', 'validation', 'tags']
    for i, s in enumerate(scenarios):
        for field in required_fields:
            if field not in s:
                errors.append(f"Cenário {i} ({s.get('id', 'sem-id')}): campo obrigatório '{field}' ausente")
    
    # 3. Verificar métricas >= 4
    for s in scenarios:
        metrics = s.get('metrics', [])
        if len(metrics) < 4:
            warnings.append(f"Cenário {s.get('id')}: apenas {len(metrics)} métricas (mínimo 4)")
    
    # 3. Verificar tags >= 2
    for s in scenarios:
        tags = s.get('tags', [])
        if len(tags) < 2:
            warnings.append(f"Cenário {s.get('id')}: apenas {len(tags)} tags (mínimo 2)")
    
    # 4. Verificar padrão de ID
    id_pattern = re.compile(r'^[a-z]+-[a-z0-9-]+$')
    for s in scenarios:
        if s.get('id') and not id_pattern.match(s['id']):
            warnings.append(f"Cenário {s.get('id')}: ID não segue padrão (letras minúsculas, números, hífens)")
    
    # 4. Verificar categorias válidas
    valid_categories = {
        'build', 'buildkit', 'run', 'compose', 'network', 'volume', 'security',
        'resources', 'logging', 'registry', 'health', 'plugin', 'swarm'
    }
    for s in scenarios:
        cat = s.get('category')
        if cat and cat not in valid_categories:
            warnings.append(f"Cenário {s.get('id')}: categoria '{cat}' não reconhecida")
    
    # 5. Verificar WP mapping (A/B/C/D/E)
    wp_mapping = {
        'A': ['build', 'buildkit'],
        'B': ['security', 'resources', 'health'],
        'C': ['network', 'volume', 'plugin', 'swarm'],
        'D': ['registry'],
        'E': ['build', 'compose', 'volume', 'network', 'logging']
    }
    
    wp_counts = {k: 0 for k in wp_mapping}
    for s in scenarios:
        cat = s.get('category')
        for wp, cats in wp_mapping.items():
            if s.get('category') in cats:
                wp_counts[wp] += 1
    
    print(f"Distribuição por WP:")
    for wp, count in wp_counts.items():
        print(f"  WP-{wp}: {count} cenarios")
    
    # 5. Verificar honest_gated
    honest_gated = sum(1 for s in scenarios if s.get('honest_gated'))
    print(f"Cenários honest-gated: {honest_gated}")
    
    print()
    if errors:
        print("❌ ERROS:")
        for e in errors:
            print(f"  - {e}")
    else:
        print("✓ Todos os checks obrigatórios passaram")
    
    if warnings:
        print()
        print("⚠️  AVISOS:")
        for w in warnings:
            print(f"  - {w}")
    
    print()
    if errors:
        print("❌ CONTRATO INVÁLIDO")
        return 1
    else:
        print("✓ CONTRATO VÁLIDO")
        return 0

if __name__ == '__main__':
    sys.exit(validate_contract())
