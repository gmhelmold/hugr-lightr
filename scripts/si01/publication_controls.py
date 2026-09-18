"""Causal publication witnesses on an archived disposable source tree.

Only an exact named assertion after successful compilation is a causal kill.
No Store/CLI activation and no mutation of the authoring worktree.
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
BASE = 'crates/lightr-store/src/store/'
PREFIX = 'store::foundation::'
CASES = [
    ('file-barrier-bypass', 'cas/preparation.rs',
     '        sync(&self.file)', '        let _ = sync; Ok(())',
     'publication_failure_tests::publication_native_file_barrier_failure_is_not_ignored',
     'called `Result::unwrap_err()` on an `Ok` value'),
    ('directory-barrier-bypass', 'foundation/lease_io.rs',
     '        sync(&self._handle)?;', '        let _ = sync;',
     'publication_failure_tests::publication_native_directory_barrier_failure_is_not_ignored',
     'called `Result::unwrap_err()` on an `Ok` value'),
    ('existence-only-reuse', 'foundation/publication.rs',
     '    if ready {', '    if existing.is_some() {',
     'publication_tests::publication_legacy_object_is_rewritten_not_resynced',
     'requalification must install a freshly written file'),
    ('replace-confirmed-object', 'foundation/publication.rs',
     '    if ready {', '    if false {',
     'publication_tests::publication_cross_lease_reuse_keeps_payload_inode_and_rebuilds_receipt',
     'confirmed payload must never be replaced'),
    ('staged-name-identity-bypass', 'cas/preparation.rs',
     '    pub(crate) fn install(&mut self, destination: &Path) -> io::Result<()> {\n        crate::store::foundation::verify_file_path(&self.file, &self._owned.payload())?;',
     '    pub(crate) fn install(&mut self, destination: &Path) -> io::Result<()> {',
     'publication_failure_tests::publication_replaced_stage_name_cannot_install_unverified_bytes',
     'called `Result::unwrap_err()` on an `Ok` value'),
]
BORROW = """use lightr_store::store::foundation::{StoreLocks, Wait};
use std::{path::Path, io::Cursor};
pub fn example(root: &Path) {
    let locks = StoreLocks::open_existing(root).unwrap();
    let lease = locks.shared(Wait::Try).unwrap();
    let proof = lease.prepare_reader(&mut Cursor::new(b"bytes"), None, Wait::Try, [0;16]).unwrap();
    RELEASE
}
"""

def require(ok, message):
    if not ok:
        raise ValueError(message)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    git = lambda *a: subprocess.check_output(['git', *a], cwd=ROOT, text=True).strip()
    receipt = {'schema': 1, 'checkout_sha': git('rev-parse', 'HEAD'),
               'tree': git('rev-parse', 'HEAD^{tree}'), 'controls': [],
               'production_protocol_enabled': False, 'runtime_qualified': False}
    def run(argv, root, label, limit=600):
        with (out/(label+'.log')).open('wb') as log:
            process = subprocess.Popen(argv, cwd=root, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                process.wait(timeout=limit)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=30)
                raise RuntimeError('watchdog is not a causal kill: '+label)
        return process.returncode, (out/(label+'.log')).read_text(errors='replace')
    def suite(root, label):
        code, text = run(['cargo', '+1.96.0', 'test', '--locked', '-p', 'lightr-store', '--lib',
                          PREFIX + 'publication', '--', '--test-threads=2'], root, label)
        require(code == 0 and re.search(r'^test result: ok\. 29 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out', text, re.M), label+' publication suite absent/failed')
    try:
        require(not git('status', '--porcelain', '--untracked-files=no'), 'dirty authoring inputs')
        archive = subprocess.check_output(['git', 'archive', '--format=zip', 'HEAD'], cwd=ROOT)
        (out/'source.zip').write_bytes(archive)
        with tempfile.TemporaryDirectory(prefix='si01-publication-controls-') as temp:
            root = Path(temp).resolve()
            with zipfile.ZipFile(io.BytesIO(archive)) as z:
                for member in z.infolist():
                    require((root/member.filename).resolve().is_relative_to(root), 'unsafe archive entry')
                z.extractall(root)
            suite(root, 'pristine')
            for label, file, old, new, test, assertion in CASES:
                path = root/BASE/file
                original = path.read_text()
                require(original.count(old) == 1, 'mutation seam drift: '+label)
                path.write_text(original.replace(old, new, 1))
                try:
                    code, _ = run(['cargo', '+1.96.0', 'test', '--locked', '-p', 'lightr-store', '--lib', '--no-run'], root, label+'-build')
                    require(code == 0, 'mutant failed compilation: '+label)
                    code, text = run(['cargo', '+1.96.0', 'test', '--locked', '-p', 'lightr-store', '--lib', PREFIX+test, '--', '--exact'], root, label+'-test', 60)
                    require(code == 101 and re.search(r'test result: FAILED\. 0 passed; 1 failed; 0 ignored; 0 measured; \d+ filtered out', text)
                            and 'test '+PREFIX+test+' ... FAILED' in text and assertion in text,
                            'wrong/missing assertion or empty suite: '+label)
                    receipt['controls'].append({'id': label, 'test': PREFIX+test, 'build_exit': 0,
                                                'test_exit': 101, 'status': 'KILLED_CAUSALLY'})
                finally:
                    path.write_text(original)
            suite(root, 'restored')
            code, text = run(['cargo', '+1.96.0', 'build', '--locked', '-p', 'lightr-store', '--message-format=json'], root, 'proof-library')
            require(code == 0, 'proof library did not build')
            libraries, directories = [], set()
            for line in text.splitlines():
                try: artifact = json.loads(line)
                except ValueError: continue
                if artifact.get('reason') == 'compiler-artifact':
                    directories.update(Path(f).parent for f in artifact['filenames'] if f.endswith(('.rlib','.rmeta','.so','.dylib')))
                    if artifact.get('target',{}).get('name') == 'lightr_store':
                        libraries.extend(Path(f) for f in artifact['filenames'] if f.endswith('.rlib'))
            require(len(libraries) == 1, 'missing/ambiguous Store library')
            library = libraries[0]
            search = [arg for directory in sorted(directories) for arg in ('-L','dependency='+str(directory))]
            receipt['proof_library_sha256'] = hashlib.sha256(library.read_bytes()).hexdigest()
            for label, release, expected in [
                ('valid-proof', 'assert!(!proof.is_empty()); drop(proof); drop(lease);', 0),
                ('invalid-early-release', 'drop(lease); assert!(!proof.is_empty());', 1),
            ]:
                source = root/(label+'.rs')
                source.write_text(BORROW.replace('RELEASE',release))
                (out/(label+'.rs')).write_text(source.read_text())
                code, text = run(['rustc', '+1.96.0', '--crate-name', 'proof_borrow_witness', '--crate-type=lib',
                                  '--edition=2021', '--emit=metadata', '--error-format=json',
                                  '--extern','lightr_store='+str(library),*search,str(source)],root,label)
                errors = []
                for line in text.splitlines():
                    try: diagnostic=json.loads(line)
                    except ValueError: continue
                    if diagnostic.get('level')=='error' and diagnostic.get('code'):
                        errors.append(diagnostic['code']['code'])
                require(code==expected and (not errors if expected==0 else errors==['E0505']), 'wrong proof lifetime result: '+label)
            receipt['proof_lifetime'] = {'valid_exit':0,'invalid_exit':1,'error_code':'E0505'}
        receipt['status']='PUBLICATION_CONTROLS_PASSED'
    except Exception as error:
        receipt['status']='FAILED'
        receipt['error']=str(error)
    receipt['artifacts']=[{'path':f.name,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()} for f in sorted(out.iterdir()) if f.is_file()]
    (out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(receipt,indent=2))
    return 0 if receipt['status']=='PUBLICATION_CONTROLS_PASSED' else 1

if __name__=='__main__':
    raise SystemExit(main())
