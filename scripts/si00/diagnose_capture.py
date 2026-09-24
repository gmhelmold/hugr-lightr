"""Bounded observations of the real legacy CLI, not a runtime acceptance gate.

Synthetic source/home/destination paths are disjoint and operation-owned. The
existing baseline gate remains independent and red on its failures. Observed
failures are retained instead of retried until green. No production code changes.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import subprocess
import sys
import tempfile

from evidence import sha256
from fixtures import identity, tree

ROOT = Path(__file__).resolve().parents[2]


def make_case(source: Path, case: str) -> None:
    source.mkdir()
    if case == 'ordinary':
        (source / 'payload').write_bytes(b'si00-diagnostic\x00\xff\n')
    elif case == 'literal-backslash-file':
        (source / 'literal\\backslash').write_bytes(b'literal-path-only\n')
    elif case == 'ordinary-dangling-link':
        (source / 'link').symlink_to('absent/leaf')
    elif case == 'literal-backslash-link':
        (source / 'link').symlink_to('absent\\leaf')
    else:
        raise ValueError('unknown diagnostic case')


def classify(snapshot: dict, hydrate: dict | None, expected: list, actual: list | None) -> str:
    if snapshot['exit']:
        return 'SNAPSHOT_REJECTED'
    if hydrate is None or hydrate['exit']:
        return 'HYDRATE_REJECTED'
    return 'ROUNDTRIP_MATCH' if expected == actual else 'TREE_MISMATCH'


def observe(binary: Path, out: Path, storm_attempts: int) -> dict:
    if sys.platform != 'linux':
        raise ValueError('this diagnostic requires native Linux, not an emulated profile')
    out.mkdir(parents=True, exist_ok=False)
    kept = out / 'lightr-baseline'
    shutil.copy2(binary, kept)
    if sha256(binary) != sha256(kept):
        raise ValueError('diagnostic executable copy mismatch')
    result = {'schema': 1, 'kind': 'legacy-cli-observation-not-acceptance',
              'checkout_sha': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
              'source_tree': subprocess.check_output(['git', 'rev-parse', 'HEAD^{tree}'], cwd=ROOT, text=True).strip(),
              'binary_sha256': sha256(kept), 'runtime_qualified': False,
              'machine': platform.machine(), 'run_id': os.environ.get('GITHUB_RUN_ID'),
              'cases': [], 'commands': [], 'status': 'STARTED'}
    trace = shutil.which('strace')
    result['trace_capability'] = 'STRACE_FILE_SYSCALLS' if trace else 'UNAVAILABLE'

    def run(home: Path, label: str, argv: list[str]) -> dict:
        launch = [str(kept), '--json', *argv]
        if trace:
            launch = [trace, '-f', '-s', '512', '-e', 'trace=%file,fsync,fdatasync',
                      '-o', str(out / (label + '.trace')), *launch]
        cmd = {'argv': launch, 'label': label}
        env = dict(os.environ, LIGHTR_HOME=str(home), RAYON_NUM_THREADS='4')
        with (out / (label + '.stdout')).open('wb') as stdout, (out / (label + '.stderr')).open('wb') as stderr:
            process = subprocess.Popen(launch, cwd=ROOT, env=env, stdout=stdout, stderr=stderr, start_new_session=True)
            try:
                cmd['exit'] = process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=10)
                cmd['exit'] = 124
        result['commands'].append(cmd)
        return cmd

    try:
        with tempfile.TemporaryDirectory(prefix='si00-owned-diagnostic-', dir=out) as temp:
            work = Path(temp)
            for case in ('ordinary', 'literal-backslash-file', 'ordinary-dangling-link', 'literal-backslash-link'):
                root = work / case
                root.mkdir()
                source, home, dest = root / 'source', root / 'home', root / 'restored'
                make_case(source, case)
                home.mkdir()
                expected = tree(source)
                first = run(home, case + '-snapshot', ['snapshot', '--dir', str(source), '--name', '@si00/diag'])
                second = None
                actual = None
                if first['exit'] == 0:
                    second = run(home, case + '-hydrate', ['hydrate', str(dest), '--name', '@si00/diag', '--verify'])
                    if second['exit'] == 0:
                        actual = tree(dest)
                row = {'case': case, 'snapshot_exit': first['exit'],
                       'hydrate_exit': None if second is None else second['exit'],
                       'source_unchanged': expected == tree(source),
                       'expected': expected, 'actual': actual,
                       'observation': classify(first, second, expected, actual)}
                result['cases'].append(row)
                print(case, row['observation'], flush=True)

            # Delimited attempt count is fixed before execution. No retries of a
            # failing attempt; a pass cannot erase the retained historical F1.
            storm = {'requested_attempts': storm_attempts, 'completed': 0, 'first_failure': None,
                     'fixture': '200 identical 1KiB files; Rayon=4; new store each attempt'}
            result['duplicate_storm'] = storm
            source = work / 'storm-source'
            source.mkdir()
            for i in range(200):
                (source / f'f{i:03}.bin').write_bytes(b'x' * 1024)
            expected = tree(source)
            storm['tree_sha256'] = identity(expected)
            for i in range(storm_attempts):
                home = work / f'storm-home-{i}'
                home.mkdir()
                cmd = run(home, f'storm-{i:02}', ['snapshot', '--dir', str(source), '--name', '@si00/storm'])
                if cmd['exit']:
                    storm['first_failure'] = {'attempt': i, 'exit': cmd['exit']}
                    break
                storm['completed'] += 1
            storm['source_unchanged'] = tree(source) == expected
        controls = [r for r in result['cases'] if r['case'] in ('ordinary', 'ordinary-dangling-link')]
        if len(controls) != 2 or any(r['observation'] != 'ROUNDTRIP_MATCH' for r in controls):
            result['status'] = 'INVALID_POSITIVE_CONTROL'
        elif not all(r['source_unchanged'] for r in result['cases']) or not storm['source_unchanged']:
            result['status'] = 'INVALID_SOURCE_CHANGED'
        else:
            result['status'] = 'OBSERVATIONS_COLLECTED'
    except Exception as exc:
        result['status'] = 'DIAGNOSTIC_FAILED'
        result['error'] = f'{type(exc).__name__}: {exc}'
    finally:
        result['artifacts'] = [{'path': p.name, 'sha256': sha256(p)} for p in sorted(out.iterdir()) if p.is_file()]
        (out / 'diagnostic.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--storm-attempts', type=int, default=20, choices=range(1, 21))
    args = parser.parse_args()
    result = observe(args.binary.resolve(), args.out.resolve(), args.storm_attempts)
    return 0 if result['status'] == 'OBSERVATIONS_COLLECTED' else 1


if __name__ == '__main__':
    sys.exit(main())
