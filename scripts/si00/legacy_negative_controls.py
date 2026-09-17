"""Compile causal regression mutants in a disposable source copy, never in the candidate."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[2]
SCAN = 'crates/lightr-index/src/index/scan.rs'
TEMP = 'crates/lightr-store/src/store/cas/owned_temp.rs'
PATH_TEST = 'index::scan::path_tests::snapshot_preserves_literal_backslash_files_and_exact_link_targets'
TEMP_TEST = 'store::cas::owned_temp::tests::forced_collision_never_claims_or_cleans_another_allocation'


def run(out: Path) -> int:
    out.mkdir(parents=True, exist_ok=False)
    results = []
    mutants = [
        ('path-rewrite', SCAN, 'raw_relative.to_owned()', "raw_relative.replace('\\\\', \"/\")", 'lightr-index', PATH_TEST),
        ('target-rewrite', SCAN, '.to_owned();\n            candidates.push(WalkCandidate {', ".replace('\\\\', \"/\");\n            candidates.push(WalkCandidate {", 'lightr-index', PATH_TEST),
        ('unreserved-directory', TEMP, 'builder.recursive(false);', 'builder.recursive(true);', 'lightr-store', TEMP_TEST),
    ]
    with tempfile.TemporaryDirectory(prefix='si00-negative-', dir=os.environ.get('RUNNER_TEMP')) as temp:
        work = Path(temp)
        archive = work / 'source.tar'
        subprocess.run(['git', 'archive', '--format=tar', '-o', str(archive), 'HEAD'], cwd=ROOT, check=True)
        source = work / 'source'
        source.mkdir()
        with tarfile.open(archive) as tar:
            tar.extractall(source, filter='data')
        for name, relative, old, new, crate, test in mutants:
            path = source / relative
            before = path.read_bytes()
            text = before.decode('utf-8')
            if text.count(old) != 1:
                raise ValueError(f'{name}: expected one mutation site, found {text.count(old)}')
            mutated = text.replace(old, new).encode('utf-8')
            path.write_bytes(mutated)
            (out / (name + '.mutation.json')).write_text(json.dumps({
                'path': relative, 'before_sha256': hashlib.sha256(before).hexdigest(),
                'after_sha256': hashlib.sha256(mutated).hexdigest(), 'old': old, 'new': new,
            }, indent=2) + '\n')
            argv = ['cargo', '+1.96.0', 'test', '--locked', '-p', crate, '--lib', test, '--', '--exact']
            try:
                with (out / (name + '.log')).open('wb') as log:
                    proc = subprocess.Popen(argv, cwd=source, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                    try:
                        code = proc.wait(timeout=300)
                    except subprocess.TimeoutExpired:
                        os.killpg(proc.pid, signal.SIGKILL)
                        proc.wait(timeout=30)
                        code = 124
                logtext = (out / (name + '.log')).read_text(encoding='utf-8', errors='replace')
                killed = (code == 101 and f'test {test} ... FAILED' in logtext and
                          re.search(r'test result: FAILED\. 0 passed; 1 failed; 0 ignored; 0 measured;', logtext) is not None)
                results.append({'mutant': name, 'argv': argv, 'exit': code,
                                'status': 'KILLED_BY_NAMED_TEST' if killed else 'INVALID_OR_SURVIVED'})
            finally:
                path.write_bytes(before)
            if not killed:
                break
    receipt = {'schema': 1, 'checkout_sha': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
               'mutants': results, 'all_killed': len(results) == 3 and all(x['status'] == 'KILLED_BY_NAMED_TEST' for x in results)}
    (out / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    return 0 if receipt['all_killed'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, required=True)
    raise SystemExit(run(parser.parse_args().out.resolve()))
