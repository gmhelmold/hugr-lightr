"""Compile exact metadata defects only in disposable source; retain every result."""
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
PREFIX = 'store::foundation::metadata::'
BASE = 'crates/lightr-store/src/store/'
CASES = [
    ('checksum', BASE+'foundation/ready.rs',
     'Digest::of_bytes(&input).0.as_slice() != &tail[length..]', 'false',
     PREFIX+'readiness_tests::every_wire_byte_participates_in_validation', 'byte '),
    ('ignored-flush', BASE+'foundation/metadata/publish.rs',
     'ops.sync_file(&file)', 'ops.sync_file(&file).or(Ok(()))',
     PREFIX+'publish_tests::checked_metadata_flush_failure_preserves_prior_destination',
     'called `Result::unwrap_err()` on an `Ok` value'),
    ('wrong-visibility', BASE+'foundation/metadata/publish.rs',
     'Visible::InstalledUnconfirmed', 'Visible::Unchanged',
     PREFIX+'publish_tests::directory_failure_keeps_installed_bytes_and_reports_uncertainty',
     'assertion `left == right` failed'),
    ('unchecked-staging', BASE+'foundation/metadata/publish.rs',
     'bytes[offset..offset + count] != buffer[..count]', 'false',
     PREFIX+'publish_tests::checked_metadata_verifies_actual_staged_bytes',
     'called `Result::unwrap_err()` on an `Ok` value'),
    ('staged-identity', BASE+'foundation/metadata/publish.rs',
     'crate::store::foundation::verify_file_path(&file, &temporary)', 'Ok::<(), io::Error>(())',
     PREFIX+'robustness_tests::metadata_replaced_staging_name_cannot_install_other_bytes',
     'called `Result::unwrap_err()` on an `Ok` value'),
    ('reader-count', BASE+'foundation/metadata/readiness.rs', 'if count > limit {', 'if false {',
     PREFIX+'robustness_tests::readiness_capture_rejects_invalid_reader_count_without_panicking',
     'invalid reader count must produce an error, not panic'),
    ('cleanup-disarm', BASE+'cas/owned_temp.rs', 'self.cleanup_done = true;', 'self.cleanup_done = false;',
     'store::cas::owned_temp::tests::successful_explicit_cleanup_does_not_clean_a_successor_allocation',
     'called `Result::unwrap()` on an `Err` value'),
]

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, default=ROOT/'si01-evidence'/'negative-controls')
    out = parser.parse_args().out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    git = lambda *args: subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()
    receipt = {'schema': 2, 'source_sha': git('rev-parse', 'HEAD'),
               'tree': git('rev-parse', 'HEAD^{tree}'), 'cases': [],
               'production_protocol_enabled': False, 'runtime_qualified': False}
    def need(ok, message):
        if not ok:
            raise ValueError(message)
    def run(argv, root, name, timeout=600):
        log = out/(name+'.log')
        with log.open('wb') as output:
            process = subprocess.Popen(argv, cwd=root, stdout=output,
                                       stderr=subprocess.STDOUT, start_new_session=True)
            try:
                result = process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=30)
                raise RuntimeError('watchdog is not a causal kill: '+name)
        return result, log.read_text(errors='replace')
    try:
        need(not git('status', '--porcelain', '--untracked-files=no'), 'tracked source is dirty')
        raw = subprocess.check_output(['git', 'archive', '--format=zip', 'HEAD'], cwd=ROOT)
        (out/'source.zip').write_bytes(raw)
        with tempfile.TemporaryDirectory(prefix='si01-metadata-controls-') as temporary:
            root = Path(temporary).resolve()
            with zipfile.ZipFile(io.BytesIO(raw)) as archive:
                for item in archive.infolist():
                    target = root/item.filename
                    need(target.resolve().is_relative_to(root), 'unsafe archive path')
                    if item.is_dir():
                        target.mkdir(parents=True, exist_ok=True)
                    else:
                        target.parent.mkdir(parents=True, exist_ok=True)
                        target.write_bytes(archive.read(item))
                        target.chmod((item.external_attr >> 16) & 0o777 or 0o644)
            cargo = ['cargo', '+1.96.0', 'test', '--locked', '-p', 'lightr-store', '--lib']
            for label, filename, old, new, test, assertion in CASES:
                path = root/filename
                original = path.read_text()
                need(original.count(old) == 1, 'mutation anchor drift: '+label)
                argv = cargo+[test, '--', '--exact', '--test-threads=1']
                code, text = run(argv, root, label+'-pristine')
                need(code == 0 and 'test '+test+' ... ok' in text
                     and '1 passed; 0 failed; 0 ignored;' in text, 'pristine witness absent/failed: '+label)
                changed = original.replace(old, new, 1)
                row = {'name': label, 'test': test, 'status': 'STARTED',
                       'original_sha256': hashlib.sha256(original.encode()).hexdigest(),
                       'mutant_sha256': hashlib.sha256(changed.encode()).hexdigest()}
                receipt['cases'].append(row)
                try:
                    path.write_text(changed)
                    row['build_exit'], _ = run(cargo+['--no-run'], root, label+'-build')
                    need(row['build_exit'] == 0, 'noncompiling mutant: '+label)
                    row['test_exit'], text = run(argv, root, label+'-test', 60)
                    need(row['test_exit'] == 101 and 'test '+test+' ... FAILED' in text
                         and '0 passed; 1 failed; 0 ignored;' in text and assertion in text,
                         'wrong/absent causal assertion: '+label)
                    row['status'] = 'KILLED_BY_REQUIRED_TEST'
                finally:
                    path.write_text(original)
                code, text = run(argv, root, label+'-restored')
                need(code == 0 and '1 passed; 0 failed; 0 ignored;' in text,
                     'restored witness absent/failed: '+label)
            code, text = run(cargo+[PREFIX, '--', '--test-threads=4'], root, 'restored-suite')
            need(code == 0 and '23 passed; 0 failed; 0 ignored;' in text, 'final metadata suite failed')
        receipt['status'] = 'METADATA_CONTROLS_PASSED'
    except Exception as error:
        receipt['status'] = 'FAILED'
        receipt['error'] = str(error)
    receipt['artifacts'] = [{'path': f.name, 'sha256': hashlib.sha256(f.read_bytes()).hexdigest()}
                            for f in sorted(out.iterdir()) if f.is_file()]
    (out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n')
    print(json.dumps(receipt, indent=2))
    return 0 if receipt['status'] == 'METADATA_CONTROLS_PASSED' else 1

if __name__ == '__main__':
    raise SystemExit(main())
