"""Validate all native profile artifacts against this checkout, not themselves."""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

from evidence import load_json, require, validate_native
from inventory import fingerprint

ROOT = Path(__file__).resolve().parents[2]


def aggregate(root: Path, expected: dict, policy: dict) -> dict:
    require(bool(policy.get('profiles')), 'empty required profile policy')
    receipts = sorted(root.glob('*/receipt.json'))
    require(len(receipts) == len(policy['profiles']), 'missing or duplicate native profile artifacts')
    profiles, rows = set(), []
    for path in receipts:
        receipt = load_json(path)
        profile = receipt.get('profile')
        require(profile in policy['profiles'] and profile not in profiles, 'unknown or duplicate native profile')
        profiles.add(profile)
        result = validate_native(receipt, path.parent, dict(expected, profile=profile), policy)
        rows.append(dict(profile=profile, **result, capabilities=receipt.get('capabilities', {})))
    require(profiles == set(policy['profiles']), 'profile matrix incomplete')
    return dict(status='SI00_NATIVE_MATRIX_VERIFIED', runtime_qualified=False,
                profiles=rows, checkout_sha=expected['checkout_sha'], input_fingerprint=expected['input_fingerprint'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    expected = {'checkout_sha': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                'input_fingerprint': fingerprint(ROOT),
                'checkout_parents': subprocess.check_output(['git', 'show', '-s', '--format=%P', 'HEAD'], cwd=ROOT, text=True).split()}
    policy = load_json(ROOT/'docs/plans/snapshot-integrity/bootstrap/expected-tests.json')
    result = aggregate(args.artifacts, expected, policy)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
    print(json.dumps(result, indent=2))
