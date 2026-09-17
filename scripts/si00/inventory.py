"""Reproducible conservative symbol inventory, not a resolved Rust call graph.

Indexes every tracked Rust file (including tests/macros). Full symbol occurrences
are retained, so alias/import/definition/comment candidates cannot disappear due
to a fragile pseudo-parser. Reviewed call paths live in the adjacent seam record.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

SYMBOLS = (
    'atomic_write', 'fsync_dir', 'ingest_file', 'put_bytes', 'get_bytes',
    'materialize_file', 'remove_object', 'exists', 'ref_put', 'ref_get',
    'ref_log', 'ref_remove', 'list_refs', 'ac_put', 'ac_get', 'list_ac',
    'image_config_put', 'image_config_get', 'image_manifest_put',
    'image_manifest_get', 'copy_image_sidecars', 'remove_image_sidecars',
    'list_image_reachable_blobs', 'write_guard', 'gc_guard', 'snapshot',
    'hydrate', 'hydrate_verified', 'load_for', 'save_for', 'index_path_for',
    'index_dir', 'probe_rung', 'cow_copy_file', 'parse_lrr1', 'gc',
)
MATCH = re.compile(r'\b(' + '|'.join(SYMBOLS) + r')\b')


def tracked(root: Path) -> list[str]:
    return subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).decode().rstrip('\0').split('\0')


def input_manifest(root: Path) -> list[dict]:
    # Include embedded JSON/Swift/C/shell/fixtures too, not only Rust extensions.
    # Only data-only SI00 review/receipt files are excluded from execution inputs.
    def receipt_only(path: str) -> bool:
        return (path.startswith('docs/plans/snapshot-integrity/evidence/') or
                path == 'docs/plans/snapshot-integrity/DISPATCH.md' or
                path == 'docs/adr/0020-snapshot-integrity-activation.md' or
                (path.startswith('docs/plans/snapshot-integrity/bootstrap/') and path.endswith('.md')))
    paths = [p for p in tracked(root) if p and not receipt_only(p)]
    return [{'path': p, 'sha256': hashlib.sha256((root/p).read_bytes()).hexdigest()} for p in sorted(paths)]


def fingerprint(root: Path) -> str:
    return hashlib.sha256(json.dumps(input_manifest(root), sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def collect(root: Path) -> dict:
    files, hits = [], []
    for name in tracked(root):
        if not name.endswith('.rs'):
            continue
        raw = (root/name).read_bytes()
        files.append({'path': name, 'sha256': hashlib.sha256(raw).hexdigest()})
        for line_no, line in enumerate(raw.decode('utf-8').splitlines(), 1):
            for hit in MATCH.finditer(line):
                symbol = hit.group()
                preceding = line[:hit.start()]
                kind = ('comment' if line.lstrip().startswith(('//', '*')) else
                        'definition' if re.search(r'\bfn\s*$', preceding) else
                        'call_candidate' if re.match(r'\s*(?:\(|::<)', line[hit.end():]) else
                        'reference_or_import')
                hits.append({'path': name, 'line': line_no, 'symbol': symbol, 'kind': kind,
                             'text': line.strip()})
    return {'schema': 1, 'method': 'all-tracked-rust-symbol-occurrences',
            'limitation': 'Conservative syntactic candidates; not type resolution or indirect-call proof.',
            'files': files, 'symbols': list(SYMBOLS), 'occurrences': hits}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    data = collect(root)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(data, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'rust_files': len(data['files']), 'symbol_occurrences': len(data['occurrences'])}))
