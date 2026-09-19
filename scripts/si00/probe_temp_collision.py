"""Manual Linux/glibc allocator witness against a supplied, hashed Lightr binary.

This diagnostic deliberately freezes wall time and synchronizes CAS temp opens
in its subprocess only. It is not a storage qualifier or historical attribution.
Builds a test-only interposer with the local C compiler; no production changes.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import sys
import tempfile

from evidence import sha256


def run_command(argv: list[str], env: dict | None = None) -> subprocess.CompletedProcess:
    proc = subprocess.Popen(argv, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    try:
        stdout, stderr = proc.communicate(timeout=30)
    except subprocess.TimeoutExpired:
        os.killpg(proc.pid, signal.SIGKILL)
        stdout, stderr = proc.communicate(timeout=10)
        raise RuntimeError('instrumentation timeout; INVALID, not a causal failure')
    return subprocess.CompletedProcess(argv, proc.returncode, stdout, stderr)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--expected-binary-sha256', required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != 'linux' or platform.libc_ver()[0] != 'glibc':
        parser.error('native Linux/glibc is required; unsupported is not passing evidence')
    binary = args.binary.resolve()
    if sha256(binary) != args.expected_binary_sha256:
        parser.error('binary identity mismatch')
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    cc = shutil.which('cc')
    if cc is None:
        parser.error('C compiler unavailable; diagnostic not exercised')
    source = Path(__file__).parent / 'probes/force_temp_collision.c'
    copied = out / source.name
    shutil.copy2(source, copied)
    library = out / 'force_temp_collision.so'
    compilation = run_command([cc, '-Wall', '-Wextra', '-Werror', '-fPIC', '-shared',
                               '-o', str(library), str(copied), '-ldl', '-pthread'])
    (out / 'compile.stdout').write_bytes(compilation.stdout)
    (out / 'compile.stderr').write_bytes(compilation.stderr)
    if compilation.returncode:
        raise RuntimeError('instrumentation did not compile; no runtime result')
    result = {'schema': 1, 'kind': 'controlled-collision-not-qualification',
              'binary_sha256': sha256(binary), 'interposer_sha256': sha256(library),
              'source_sha256': sha256(copied), 'os': platform.platform(),
              'compiler': run_command([cc, '--version']).stdout.decode(),
              'runtime_qualified': False, 'historical_cause_attributed': False, 'cases': []}
    try:
        with tempfile.TemporaryDirectory(prefix='owned-', dir=out) as temporary:
            work = Path(temporary)
            source_tree = work / 'source'
            source_tree.mkdir()
            for i in range(200):
                (source_tree / f'f{i:03}').write_bytes(b'x' * 1024)
            expected = {p.name: sha256(p) for p in source_tree.iterdir()}
            for label, workers, inject in [('ordinary-four', 4, False),
                                           ('fixed-clock-one', 1, True),
                                           ('fixed-clock-four', 4, True)]:
                home = work / label
                home.mkdir()
                env = dict(os.environ, LIGHTR_HOME=str(home), RAYON_NUM_THREADS=str(workers))
                env.pop('LD_PRELOAD', None)
                env.pop('SI00_COLLISION_PARTICIPANTS', None)
                if inject:
                    env.update(LD_PRELOAD=str(library), SI00_COLLISION_PARTICIPANTS=str(workers))
                proc = run_command([str(binary), '--json', 'snapshot', '--dir', str(source_tree),
                                    '--name', '@si00/collision'], env)
                (out / (label + '.stdout')).write_bytes(proc.stdout)
                (out / (label + '.stderr')).write_bytes(proc.stderr)
                text = proc.stderr.decode('utf-8', errors='replace')
                opens = re.findall(r'SI00_OPEN (\d+)/(\d+) tid=(\d+) inode=(\d+) path=([^\n]+)', text)
                row = {'label': label, 'workers': workers, 'interposed': inject, 'exit': proc.returncode,
                       'opened': opens, 'barrier_complete': f'SI00_END arrived={workers} target={workers}' in text
                       and 'timeout=0' in text, 'source_unchanged': expected == {p.name: sha256(p) for p in source_tree.iterdir()},
                       'current_ref_files': len(list((home / 'store/refs').glob('*/*')))}
                result['cases'].append(row)
            normal, serial, parallel = result['cases']
            controls = normal['exit'] == serial['exit'] == 0 and serial['barrier_complete']
            valid = controls and parallel['barrier_complete'] and all(r['source_unchanged'] for r in result['cases'])
            same_inode = len(parallel['opened']) == 4 and len({o[3] for o in parallel['opened']}) == 1
            same_path = len(parallel['opened']) == 4 and len({o[4] for o in parallel['opened']}) == 1
            result['shared_staging_inode'] = same_inode
            result['shared_staging_path'] = same_path
            result['status'] = ('SHARED_STAGING_REPRODUCED' if valid and same_inode and same_path
                                else 'NOT_A_VALID_SHARED_STAGING_WITNESS')
            result['snapshot_rejected_without_current'] = parallel['exit'] != 0 and parallel['current_ref_files'] == 0
    except Exception as exc:
        result['status'] = 'INVALID_DIAGNOSTIC'
        result['error'] = str(exc)
    finally:
        result['artifacts'] = [{'path': p.name, 'sha256': sha256(p)} for p in sorted(out.iterdir()) if p.is_file()]
        (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: v for k, v in result.items() if k not in ('artifacts', 'cases')}))
    # A zero here means a measured controlled defect, never product acceptance.
    return 0 if result['status'] == 'SHARED_STAGING_REPRODUCED' else 1


if __name__ == '__main__':
    sys.exit(main())
