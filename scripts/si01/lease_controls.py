"""Compiled lease counterexamples, confined to a disposable source checkout.

A mutant must build, then fail one specified assertion/test. Compile errors,
watchdogs and empty suites are not counted as causal kills. Separately checks
that releasing the borrowed lease early produces E0505, with a valid control.
"""
from __future__ import annotations
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]
BASE = 'crates/lightr-store/src/store/foundation/'
PREFIX = 'store::foundation::lease_tests::'
CASES = [
    ('native-lock-noop', 'lease_io.rs',
     'let result = if shared {\n                file.try_lock_shared()\n            } else {\n                file.try_lock()\n            };',
     'let result = if shared { Ok::<(), TryLockError>(()) } else { Ok::<(), TryLockError>(()) };',
     'lease_shared_excludes_gc_and_preserves_lock_file', 'called `Result::unwrap_err()` on an `Ok` value'),
    ('root-identity-bypass', 'lease_io.rs', 'if identity(&file)? != self.id {', 'if false {',
     'lease_rejects_replaced_root_before_creating_lock', 'called `Result::unwrap_err()` on an `Ok` value'),
    ('cache-shared-instead-of-exclusive', 'lease.rs',
     'NativeLock::acquire(&self.0, ".si01-cache.lock", false, wait)?',
     'NativeLock::acquire(&self.0, ".si01-cache.lock", true, wait)?',
     'lease_foreign_store_exclusive_is_not_shared_cache_exclusion', 'called `Result::unwrap_err()` on an `Ok` value'),
    ('digest-order-bypass', 'lease.rs', 'keys.sort_unstable_by_key(|key| key.0);', '',
     'lease_digest_set_is_sorted_and_failed_partial_acquisition_is_released', 'assertion `left == right` failed'),
    ('partial-lock-leak', 'lease.rs',
     'guards.push(NativeLock::acquire(&dir, &key.to_hex(), false, wait)?);',
     'match NativeLock::acquire(&dir, &key.to_hex(), false, wait) { Ok(g) => guards.push(g), Err(e) => { std::mem::forget(guards); return Err(e); } }',
     'lease_digest_set_is_sorted_and_failed_partial_acquisition_is_released', 'called `Result::unwrap()` on an `Err` value'),
]

BORROW = '''use lightr_store::store::foundation::{StoreLocks, LeasedStagedFile, Wait};
use std::{path::Path, io::Cursor};
pub fn example(root: &Path) {
    let domain = StoreLocks::open_existing(root).unwrap();
    let lease = domain.shared(Wait::Try).unwrap();
    let staged = LeasedStagedFile::copy(&lease, &mut Cursor::new(b"bytes"), None, [0;16]).unwrap();
    RELEASE
}
'''

def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument('--out', required=True, type=Path)
    args = p.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    git = lambda *a: subprocess.check_output(['git', *a], cwd=ROOT, text=True).strip()
    receipt = {'schema': 1, 'checkout_sha': git('rev-parse', 'HEAD'),
               'tree': git('rev-parse', 'HEAD^{tree}'), 'controls': [],
               'production_protocol_enabled': False, 'runtime_qualified': False}
    def run(argv, root, name, limit=600):
        with (out/(name+'.log')).open('wb') as log:
            proc = subprocess.Popen(argv, cwd=root, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                proc.wait(timeout=limit)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait(timeout=30)
                raise RuntimeError('watchdog is not a causal kill: '+name)
        return proc.returncode, (out/(name+'.log')).read_text(errors='replace')
    def need(ok, reason):
        if not ok:
            raise ValueError(reason)
    try:
        archive = subprocess.check_output(['git', 'archive', '--format=zip', 'HEAD'], cwd=ROOT)
        (out/'source.zip').write_bytes(archive)
        with tempfile.TemporaryDirectory(prefix='si01-lease-controls-') as temp:
            root = Path(temp)
            with zipfile.ZipFile(io.BytesIO(archive)) as z:
                for item in z.infolist():
                    need((root/item.filename).resolve().is_relative_to(root), 'unsafe archive path')
                z.extractall(root)
            suite = ['cargo', '+1.96.0', 'test', '--locked', '-p', 'lightr-store', '--lib', PREFIX, '--', '--test-threads=1']
            code, text = run(suite, root, 'pristine')
            need(code == 0 and re.search(r'^test result: ok\. 13 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out', text, re.M), 'pristine lease suite missing/failed')
            for label, file, old, new, test, assertion in CASES:
                path = root/BASE/file
                original = path.read_text()
                need(original.count(old) == 1, 'mutation seam drift: '+label)
                path.write_text(original.replace(old, new, 1))
                try:
                    code, _ = run(['cargo', '+1.96.0', 'test', '--locked', '-p', 'lightr-store', '--lib', '--no-run'], root, label+'-build')
                    need(code == 0, 'mutant did not compile: '+label)
                    code, text = run(['cargo', '+1.96.0', 'test', '--locked', '-p', 'lightr-store', '--lib', PREFIX+test, '--', '--exact'], root, label+'-test', 60)
                    summary = r'test result: FAILED\. 0 passed; 1 failed; 0 ignored; 0 measured; \d+ filtered out'
                    need(code == 101 and re.search(summary, text) and 'test '+PREFIX+test+' ... FAILED' in text and assertion in text,
                         'wrong/absent assertion, empty suite or timeout: '+label)
                    receipt['controls'].append({'id': label, 'test': PREFIX+test, 'build_exit': 0, 'test_exit': 101, 'status': 'KILLED_CAUSALLY'})
                finally:
                    path.write_text(original)
            code, text = run(suite, root, 'restored')
            need(code == 0 and re.search(r'^test result: ok\. 13 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out', text, re.M), 'restored lease suite failed')
            code, text = run(['cargo', '+1.96.0', 'build', '--locked', '-p', 'lightr-store', '--message-format=json'], root, 'borrow-library')
            need(code == 0, 'borrow-check library build failed')
            libs, dependencies = [], set()
            for line in text.splitlines():
                try: item = json.loads(line)
                except ValueError: continue
                if item.get('reason') == 'compiler-artifact':
                    dependencies.update(Path(f).parent for f in item['filenames'] if f.endswith(('.rlib', '.rmeta', '.so')))
                    if item.get('target', {}).get('name') == 'lightr_store':
                        libs.extend(Path(f) for f in item['filenames'] if f.endswith('.rlib'))
            need(len(libs) == 1, 'missing/ambiguous compiled Store library')
            lib = libs[0]
            search = [arg for directory in sorted(dependencies) for arg in ('-L', 'dependency='+str(directory))]
            receipt['borrow_library_sha256'] = hashlib.sha256(lib.read_bytes()).hexdigest()
            receipt['borrow_dependency_dirs'] = [str(d) for d in sorted(dependencies)]
            for name, release, expected in [
                ('valid-borrow', 'assert!(!staged.is_empty()); drop(staged); drop(lease);', 0),
                ('invalid-early-release', 'drop(lease); assert!(!staged.is_empty());', 1),
            ]:
                source = root/(name+'.rs')
                source.write_text(BORROW.replace('RELEASE', release))
                (out/(name+'.rs')).write_text(source.read_text())
                code, text = run(['rustc', '+1.96.0', '--crate-name', 'lease_borrow_witness', '--crate-type=lib', '--edition=2021',
                                  '--emit=metadata', '--error-format=json', '--extern', 'lightr_store='+str(lib),
                                  *search, str(source)], root, name)
                errors = []
                for line in text.splitlines():
                    try: d = json.loads(line)
                    except ValueError: continue
                    if d.get('level') == 'error' and d.get('code'): errors.append(d['code']['code'])
                need(code == expected and (not errors if expected == 0 else errors == ['E0505']), 'incorrect borrow witness: '+name)
            receipt['borrow_check'] = {'valid_exit': 0, 'invalid_exit': 1, 'error_code': 'E0505'}
        receipt['status'] = 'LEASE_CONTROLS_PASSED'
    except Exception as error:
        receipt['status'] = 'FAILED'
        receipt['error'] = str(error)
    receipt['artifacts'] = [{'path': f.name, 'sha256': hashlib.sha256(f.read_bytes()).hexdigest()} for f in sorted(out.iterdir()) if f.is_file()]
    (out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')
    print(json.dumps(receipt, indent=2))
    return 0 if receipt['status'] == 'LEASE_CONTROLS_PASSED' else 1

if __name__ == '__main__':
    raise SystemExit(main())
