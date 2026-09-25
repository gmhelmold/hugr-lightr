"""Fail-closed SI-00 evidence checks. This is NOT a runtime qualifier.

The caller supplies expected checkout/input identities and required tests; the
receipt cannot decide its own acceptance policy. Only stdlib is required.
"""
from __future__ import annotations

import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any


class InvalidEvidence(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise InvalidEvidence(message)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def load_json(path: Path) -> Any:
    def unique_pairs(pairs: list) -> dict:
        result = {}
        for key, value in pairs:
            require(key not in result, f'duplicate JSON key: {key}')
            result[key] = value
        return result
    return json.loads(path.read_text(encoding='utf-8'), object_pairs_hook=unique_pairs)


def artifact(root: Path, relative: str) -> Path:
    require(isinstance(relative, str) and bool(relative), 'missing artifact path')
    path = Path(relative)
    require(not path.is_absolute() and '..' not in path.parts and '\\' not in relative and ':' not in relative and path.as_posix() == relative,
            'artifact path escapes evidence directory')
    current = root.resolve()
    for part in path.parts:
        current = current / part
        require(not current.is_symlink(), 'symlink is not an evidence artifact')
    require(current.is_file(), f'artifact absent: {relative}')
    require(current.resolve().is_relative_to(root.resolve()), 'artifact escaped root')
    return current


def parse_tests(text: str) -> tuple[dict[str, str], list[int]]:
    events = re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored[^\r\n]*)\r?$', text, re.M)
    require(len({n for n, _ in events}) == len(events), 'duplicate test event')
    summaries = re.findall(
        r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;'
        r' (\d+) measured; (\d+) filtered out', text)
    require(len(summaries) == 1, 'missing/ambiguous libtest summary')
    counts = list(map(int, summaries[0]))
    statuses = dict(events)
    actual = [sum(s == 'ok' for s in statuses.values()),
              sum(s == 'FAILED' for s in statuses.values()),
              sum(s.startswith('ignored') for s in statuses.values())]
    require(counts[:3] == actual, 'summary does not match named tests')
    require(counts[3:] == [0, 0], 'measured/filtered tests cannot qualify smoke')
    return statuses, counts


def validate_native(receipt: dict, root: Path, expected: dict, policy: dict) -> dict:
    require(receipt.get('schema') == 2 and receipt.get('stage') == 'SI00_NATIVE_BASELINE',
            'unknown evidence schema/stage (not a campaign qualifier)')
    require(receipt.get('status') == 'EXECUTED', 'execution not successful')
    require(receipt.get('production_protocol_enabled') is False, 'bootstrap activated protocol')
    for key in ('checkout_sha', 'source_tree', 'input_fingerprint', 'profile'):
        require(receipt.get(key) == expected.get(key), f'wrong {key}')
    require(re.fullmatch(r'[0-9a-f]{40}', receipt['checkout_sha']) is not None, 'bad SHA')
    profile = policy['profiles'][receipt['profile']]
    require(receipt.get('rust_host') == profile['host'], 'target is not native runner host')
    require(receipt.get('machine', '').lower() in profile['machines'], 'runner architecture mismatch')
    require(receipt.get('toolchain') == '1.96.0', 'wrong pinned toolchain')
    require(receipt.get('tracked_dirty_before') is False and receipt.get('tracked_dirty_after') is False,
            'dirty source/test/workflow inputs')
    require(isinstance(receipt.get('checkout_parents'), list) and receipt['checkout_parents'], 'parents missing')
    require(receipt['checkout_parents'] == expected['checkout_parents'], 'wrong checkout parents')
    require(receipt.get('plan_commit') == policy['plan_commit'], 'wrong plan contract')
    files = receipt.get('artifacts', [])
    require(files and len({f['path'] for f in files}) == len(files), 'empty/duplicate artifact inventory')
    for f in files:
        require(sha256(artifact(root, f['path'])) == f['sha256'], 'artifact checksum mismatch')
    present = {f['path'] for f in files}
    commands = receipt.get('commands', [])
    require(commands and len({c['label'] for c in commands}) == len(commands), 'commands missing/duplicated')
    cmds = {c['label']: c for c in commands}
    require('build' in cmds and cmds['build']['exit'] == 0, 'native build failed')
    for c in commands:
        require(c.get('argv') and c['finished'] >= c['started'], 'invalid command ordering')
        require(c['stdout'] in present and c['stderr'] in present, 'raw command logs absent')
    bins = receipt.get('binaries', [])
    require({b['name'] for b in bins} == set(policy['required_tests']) and len(bins) == 2,
            'missing/duplicate native test binary')
    total = ignored = 0
    for b in bins:
        name = b['name']
        require(b['path'] in present and sha256(artifact(root, b['path'])) == b['sha256'], 'wrong binary')
        listed, tested = cmds.get('list-' + name), cmds.get('run-' + name)
        require(listed is not None and tested is not None, 'missing list/run command')
        require(listed['exit'] == 0 and tested['exit'] == 0, 'native test command failed')
        require(cmds['build']['finished'] <= listed['started'] <= listed['finished'] <= tested['started'],
                'tests precede build/inventory')
        require(listed['argv'][0].replace('\\', '/').split('/')[-1] == Path(b['path']).name and
                tested['argv'][0].replace('\\', '/').split('/')[-1] == Path(b['path']).name, 'tested binary mismatch')
        text = artifact(root, listed['stdout']).read_text(encoding='utf-8')
        names = re.findall(r'^(.+): test$', text, re.M)
        require(names and len(set(names)) == len(names), 'empty/duplicate test inventory')
        events, counts = parse_tests(artifact(root, tested['stdout']).read_text(encoding='utf-8'))
        require(set(events) == set(names), 'not all enumerated tests executed')
        extra = profile.get('required_tests', {})
        require(isinstance(extra, dict) and set(extra) <= set(policy['required_tests']),
                'invalid profile-specific required-test policy')
        specific = extra.get(name, [])
        require(isinstance(specific, list) and all(isinstance(n, str) and n for n in specific),
                'invalid profile-specific test names')
        required_names = policy['required_tests'][name] + specific
        require(len(required_names) == len(set(required_names)), 'duplicate required test policy')
        for required in required_names:
            require(events.get(required) == 'ok', f'required smoke not passed: {required}')
        for n, status in events.items():
            require(status == 'ok' or (status.startswith('ignored') and n in policy['allowed_ignored']),
                    f'unapproved skipped/failed test: {n}')
        total += counts[0]; ignored += counts[2]
    return {'status': 'NATIVE_BASELINE_VERIFIED', 'passed': total, 'ignored': ignored,
            'runtime_qualified': False, 'formatting_exit': cmds.get('fmt', {}).get('exit')}


def validate_benchmarks(result: dict, budget: dict, expected_budget_hash: str) -> None:
    """Validate a numeric comparison, never an echo-only or empty success.

    The budget digest is pinned by the caller before measuring the candidate.
    This function is delivered now; no campaign budget is fabricated by SI-00.
    """
    encoded = (json.dumps(budget, sort_keys=True, separators=(',', ':')) + '\n').encode()
    require(hashlib.sha256(encoded).hexdigest() == expected_budget_hash, 'budget not preregistered')
    require(budget.get('status') == 'REGISTERED' and bool(budget.get('approver')), 'unapproved budget')
    rows = result.get('measurements', [])
    limits = budget.get('limits', [])
    require(rows and limits, 'no benchmark evidence')
    keys = lambda xs: [(x['profile'], x['fixture'], x['metric']) for x in xs]
    require(len(set(keys(rows))) == len(rows) and len(set(keys(limits))) == len(limits) and set(keys(rows)) == set(keys(limits)), 'incomplete/duplicate metrics')
    require(result.get('measured_at', 0) > budget.get('registered_at', float('inf')), 'budget recorded too late')
    require(result.get('checkout_sha') == budget.get('candidate_sha'), 'wrong benchmark candidate')
    for row, limit in ((r, dict(zip(keys(limits), limits))[k]) for r, k in zip(rows, keys(rows))):
        samples = row.get('samples', [])
        require(type(limit['min_samples']) is int and len(samples) >= limit['min_samples'] > 0, 'insufficient samples')
        require(all(type(x) in (int, float) and math.isfinite(x) and x >= 0 for x in samples), 'invalid sample')
        ceiling = limit['maximum']
        require(type(ceiling) in (int, float) and math.isfinite(ceiling) and ceiling >= 0, 'invalid limit')
        require(row.get('unit') == limit['unit'], 'wrong unit')
        values = sorted(samples); size = len(values)
        median = values[size//2] if size % 2 else (values[size//2-1] + values[size//2])/2
        require(median <= ceiling, 'performance budget exceeded')
