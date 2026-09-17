"""Run real native baseline tests, retain binaries/logs, validate evidence.

No runtime protocol implementation, no network calls other than Cargo, no user
workspace or real Store is used. The existing ignored test remains visible.
"""
from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

from evidence import load_json, require, sha256, validate_native
from inventory import fingerprint, input_manifest
from probes import capabilities

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / 'docs/plans/snapshot-integrity/bootstrap/expected-tests.json'


def git(*args: str) -> str:
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--profile', required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve(); out.mkdir(parents=True, exist_ok=False)
    (out / 'binaries').mkdir()
    policy = load_json(POLICY)
    require(args.profile in policy['profiles'], 'unknown native profile')
    expected = {'checkout_sha': git('rev-parse', 'HEAD'), 'input_fingerprint': fingerprint(ROOT), 'profile': args.profile, 'checkout_parents': git('show', '-s', '--format=%P', 'HEAD').split()}
    receipt = dict(expected, schema=2, stage='SI00_NATIVE_BASELINE', status='STARTED',
                   plan_commit=policy['plan_commit'],
                   source_tree=git('rev-parse', 'HEAD^{tree}'), production_protocol_enabled=False,
                   machine=platform.machine(), os=platform.platform(), toolchain='1.96.0',
                   run_id=os.environ.get('GITHUB_RUN_ID'), event=os.environ.get('GITHUB_EVENT_NAME'),
                   requested_sha=os.environ.get('GITHUB_SHA'), commands=[], binaries=[], artifacts=[])
    receipt['tracked_status_before'] = git('status', '--porcelain', '--untracked-files=no')
    receipt['tracked_dirty_before'] = bool(receipt['tracked_status_before'])
    print('tracked_status_before=' + json.dumps(receipt['tracked_status_before']), flush=True)
    (out / 'input-manifest.json').write_text(json.dumps(input_manifest(ROOT), indent=2) + '\n')
    (out / 'requirements.json').write_text(json.dumps(policy, indent=2) + '\n')
    (out / 'expected.json').write_text(json.dumps(expected, indent=2) + '\n')

    def run(argv: list[str], label: str, timeout: int = 900) -> tuple[dict, str]:
        require(re.fullmatch(r'[a-zA-Z0-9_-]+', label) is not None, 'unsafe log label')
        command = {'label': label, 'argv': argv, 'cwd': str(ROOT), 'started': time.time(),
                   'stdout': label + '.stdout', 'stderr': label + '.stderr'}
        # Files instead of unbounded in-memory logs; timeout kills the process.
        with (out / command['stdout']).open('wb') as stdout, (out / command['stderr']).open('wb') as stderr:
            try:
                process = subprocess.Popen(argv, cwd=ROOT, stdout=stdout, stderr=stderr, start_new_session=os.name == 'posix')
                command['exit'] = process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                if os.name == 'posix':
                    import signal
                    os.killpg(process.pid, signal.SIGKILL)
                else:
                    subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'], stdout=stderr, stderr=stderr, timeout=30)
                process.wait(timeout=30)
                command['exit'] = 124
        command['finished'] = time.time(); receipt['commands'].append(command)
        print(label, command['exit'], flush=True)
        return command, (out / command['stdout']).read_text(encoding='utf-8', errors='replace')

    failed = False
    try:
        for label, argv in [('rustc', ['rustc', '+1.96.0', '-Vv']), ('cargo', ['cargo', '+1.96.0', '-V'])]:
            cmd, text = run(argv, label, 120); require(cmd['exit'] == 0, label + ' unavailable')
            receipt[label] = text.strip()
        host = re.search(r'^host: (.+)$', receipt['rustc'], re.M)
        require(host is not None, 'rustc host missing'); receipt['rust_host'] = host.group(1)
        require(receipt['rust_host'] == policy['profiles'][args.profile]['host'], 'non-native toolchain')
        receipt['capabilities'] = capabilities(out)
        cmd, data = run(['cargo', '+1.96.0', 'test', '--locked', '-p', 'lightr-store', '-p', 'lightr-index', '--lib', '--no-run', '--message-format=json'], 'build')
        require(cmd['exit'] == 0, 'native test build failed')
        artifacts = []
        for line in data.splitlines():
            try: row = json.loads(line)
            except ValueError: continue
            if row.get('reason') == 'compiler-artifact' and row.get('profile', {}).get('test') and row.get('executable'):
                artifacts.append(row)
        require(len(artifacts) == 2, 'expected two native libtest binaries')
        for row in artifacts:
            name = row['target']['name']; original = Path(row['executable'])
            dest = out / 'binaries' / original.name
            shutil.copyfile(original, dest); shutil.copymode(original, dest)
            require(sha256(original) == sha256(dest), 'binary copy mismatch')
            receipt['binaries'].append({'name': name, 'path': dest.relative_to(out).as_posix(), 'sha256': sha256(dest)})
            listed, _ = run([str(dest), '--list'], 'list-' + name, 60)
            tested, _ = run([str(dest), '--test-threads=1'], 'run-' + name, 300)
            if listed['exit'] or tested['exit']: failed = True
        # The pre-existing formatter failure remains an explicit failed gate.
        fmt, _ = run(['cargo', '+1.96.0', 'fmt', '--all', '--', '--check'], 'fmt', 120)
        receipt['formatting_gate'] = 'PASS' if fmt['exit'] == 0 else 'FAIL_PREEXISTING_BASELINE'
    except Exception as exc:
        receipt['error'] = f'{type(exc).__name__}: {exc}'; failed = True
    finally:
        receipt['tracked_status_after'] = git('status', '--porcelain', '--untracked-files=no')
        receipt['tracked_dirty_after'] = bool(receipt['tracked_status_after'])
        print('tracked_status_after=' + json.dumps(receipt['tracked_status_after']), flush=True)
        if fingerprint(ROOT) != expected['input_fingerprint']: failed = True
        receipt['status'] = 'EXECUTION_FAILED' if failed else 'EXECUTED'
        receipt['artifacts'] = [{'path': p.relative_to(out).as_posix(), 'sha256': sha256(p)}
                                for p in sorted(out.rglob('*')) if p.is_file() and p.name != 'receipt.json']
        try:
            receipt['validation'] = validate_native(receipt, out, expected, policy)
        except Exception as exc:
            receipt['validation'] = {'status': 'FAIL', 'error': str(exc)}; failed = True
        (out / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
        print(json.dumps({'profile': args.profile, 'validation': receipt['validation'], 'error': receipt.get('error')}), flush=True)
    return int(failed)


if __name__ == '__main__':
    sys.exit(main())
