"""Compile causal mutations in an isolated source copy; never mutate the checkout."""
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
OUT = ROOT / 'si01-evidence' / 'negative-controls'
OUT.mkdir(parents=True, exist_ok=False)
PREFIX = 'store::cas::foundation::'
CASES = [
    ('checksum', 'readiness.rs', 'checksum(&bytes[..end]).0 != bytes[end..]', 'false',
     PREFIX + 'readiness_tests::every_wire_byte_participates_in_validation'),
    ('ignored-flush', 'publish.rs', 'ops.sync_file(&file)', 'ops.sync_file(&file).or(Ok(()))',
     PREFIX + 'publish_tests::checked_metadata_flush_failure_preserves_prior_destination'),
    ('wrong-visibility', 'publish.rs', 'Visible::InstalledUnconfirmed', 'Visible::Unchanged',
     PREFIX + 'publish_tests::directory_failure_keeps_installed_bytes_and_reports_uncertainty'),
    ('unchecked-staging', 'publish.rs', 'bytes[offset..offset + count] != buffer[..count]', 'false',
     PREFIX + 'publish_tests::checked_metadata_verifies_actual_staged_bytes'),
]

def run(argv, cwd, log):
    with log.open('wb') as output:
        process = subprocess.Popen(argv, cwd=cwd, stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            return process.wait(timeout=900)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            raise RuntimeError('watchdog is not a killed mutant')

rows = []
try:
    raw = subprocess.check_output(['git', 'archive', '--format=zip', 'HEAD'], cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix='si01-mutants-') as temporary:
        root = Path(temporary)
        with zipfile.ZipFile(io.BytesIO(raw)) as archive:
            for entry in archive.infolist():
                relative = Path(entry.filename)
                assert not relative.is_absolute() and '..' not in relative.parts
                target = root / relative
                if entry.is_dir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(archive.read(entry))
                    target.chmod((entry.external_attr >> 16) & 0o777 or 0o644)
        cargo = ['cargo', '+1.96.0', 'test', '--locked', '-p', 'lightr-store', '--lib']
        for name, filename, old, new, witness in CASES:
            path = root / 'crates/lightr-store/src/store/cas/foundation' / filename
            original = path.read_text()
            assert original.count(old) == 1, f'{name}: mutation anchor drift'
            pristine = run(cargo + [witness, '--', '--exact', '--test-threads=1'], root, OUT / (name + '-pristine.log'))
            assert pristine == 0, f'{name}: pristine witness failed'
            changed = original.replace(old, new)
            row = {'name': name, 'test': witness, 'status': 'STARTED',
                   'original_sha256': hashlib.sha256(original.encode()).hexdigest(),
                   'mutant_sha256': hashlib.sha256(changed.encode()).hexdigest()}
            rows.append(row)
            try:
                path.write_text(changed)
                row['build_exit'] = run(cargo + ['--no-run'], root, OUT / (name + '-build.log'))
                assert row['build_exit'] == 0, f'{name}: non-compiling mutant is not evidence'
                row['test_exit'] = run(cargo + [witness, '--', '--exact', '--test-threads=1'], root, OUT / (name + '.log'))
                text = (OUT / (name + '.log')).read_text()
                assert row['test_exit'] == 101 and re.search(r'^test ' + re.escape(witness) + r' \.\.\. FAILED$', text, re.M)
                assert '0 passed; 1 failed; 0 ignored;' in text, f'{name}: wrong failure'
                row['status'] = 'KILLED_BY_REQUIRED_TEST'
            finally:
                path.write_text(original)
finally:
    (OUT / 'receipt.json').write_text(json.dumps({'schema': 1, 'source_sha': subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(), 'cases': rows}, indent=2)+'\n')
assert len(rows) == len(CASES) and all(r['status'] == 'KILLED_BY_REQUIRED_TEST' for r in rows)
print('Four compiling mutants killed by their exact required tests')
