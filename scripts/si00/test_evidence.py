"""Negative controls for the evidence gate; these fixtures are NOT native runs."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from evidence import InvalidEvidence, artifact, load_json, sha256, validate_benchmarks, validate_native


class NativeEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.policy = {'plan_commit': 'c'*40, 'profiles': {'native': {'host': 'test-host', 'machines': ['test-machine']}},
                       'required_tests': {'store': ['test_one'], 'index': ['test_two']}, 'allowed_ignored': {}}
        self.expected = {'checkout_sha': 'a'*40, 'input_fingerprint': 'b'*64, 'profile': 'native', 'checkout_parents': ['d'*40]}
        self.record = dict(self.expected, schema=2, stage='SI00_NATIVE_BASELINE', status='EXECUTED',
                           production_protocol_enabled=False, rust_host='test-host', machine='test-machine',
                           toolchain='1.96.0', tracked_dirty_before=False, tracked_dirty_after=False,
                           plan_commit='c'*40, commands=[], binaries=[])
        self.command('build', 0, 1, [])
        for number, (name, tests) in enumerate(self.policy['required_tests'].items()):
            path = self.root/name; path.write_bytes(b'NOT A NATIVE BINARY: validator unit fixture')
            self.record['binaries'].append({'name': name, 'path': name, 'sha256': sha256(path)})
            self.command('list-'+name, 2+number*4, 3+number*4, [name, '--list'], '\n'.join(t+': test' for t in tests)+'\n')
            text = '\n'.join('test '+t+' ... ok' for t in tests)+'\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n'
            self.command('run-'+name, 3+number*4, 4+number*4, [name], text)
        self.rehash()

    def command(self, label, start, end, argv, text=''):
        (self.root/(label+'.stdout')).write_text(text)
        (self.root/(label+'.stderr')).write_text('')
        self.record['commands'].append({'label': label, 'started': start, 'finished': end, 'argv': argv or ['cargo'],
                                       'stdout': label+'.stdout', 'stderr': label+'.stderr', 'exit': 0})

    def rehash(self):
        self.record['artifacts'] = [{'path': p.name, 'sha256': sha256(p)} for p in self.root.iterdir() if p.is_file()]

    def validate(self):
        return validate_native(self.record, self.root, self.expected, self.policy)

    def test_valid_is_only_baseline_not_qualification(self):
        self.assertFalse(self.validate()['runtime_qualified'])

    def test_wrong_identity_axes(self):
        for key, value in [('checkout_sha', 'f'*40), ('input_fingerprint', 'f'*64), ('profile', 'other'),
                           ('rust_host', 'foreign'), ('machine', 'foreign'), ('toolchain', 'stable'), ('plan_commit', 'f'*40), ('checkout_parents', ['f'*40])]:
            with self.subTest(key=key):
                old = self.record[key]; self.record[key] = value
                with self.assertRaises(InvalidEvidence): self.validate()
                self.record[key] = old

    def test_empty_suite(self):
        (self.root/'list-store.stdout').write_text('0 tests, 0 benchmarks\n'); self.rehash()
        with self.assertRaisesRegex(InvalidEvidence, 'empty'): self.validate()

    def test_missing_named_required_test(self):
        self.policy['required_tests']['store'].append('must_execute')
        with self.assertRaisesRegex(InvalidEvidence, 'required smoke'): self.validate()

    def test_filtered_suite(self):
        p=self.root/'run-store.stdout'; p.write_text(p.read_text().replace('0 filtered out', '1 filtered out')); self.rehash()
        with self.assertRaisesRegex(InvalidEvidence, 'filtered'): self.validate()

    def test_ignored_required(self):
        p=self.root/'run-store.stdout'
        p.write_text('test test_one ... ignored\ntest result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out\n'); self.rehash()
        with self.assertRaisesRegex(InvalidEvidence, 'required smoke'): self.validate()

    def test_missing_artifact(self):
        (self.root/'build.stdout').unlink()
        with self.assertRaisesRegex(InvalidEvidence, 'absent'): self.validate()

    def test_corrupt_artifact_and_binary(self):
        (self.root/'store').write_bytes(b'different')
        with self.assertRaisesRegex(InvalidEvidence, 'checksum'): self.validate()

    def test_run_before_build(self):
        self.record['commands'][0]['finished'] = 100
        with self.assertRaisesRegex(InvalidEvidence, 'precede'): self.validate()

    def test_dirty_or_activated(self):
        for key in ['tracked_dirty_before', 'tracked_dirty_after', 'production_protocol_enabled']:
            self.record[key] = True
            with self.assertRaises(InvalidEvidence): self.validate()
            self.record[key] = False

    def test_duplicate_event_and_summary_lie(self):
        p=self.root/'run-store.stdout'; original=p.read_text()
        for bad in [original+'test test_one ... ok\n', original.replace('1 passed', '2 passed')]:
            p.write_text(bad); self.rehash()
            with self.assertRaises(InvalidEvidence): self.validate()

    def test_path_escape(self):
        with self.assertRaises(InvalidEvidence): artifact(self.root, '../outside')
        with self.assertRaises(InvalidEvidence): artifact(self.root, 'C:\\outside')

    def test_duplicate_json_keys(self):
        p=self.root/'duplicate.json'; p.write_text('{"status": "FAIL", "status": "PASS"}')
        with self.assertRaises(InvalidEvidence): load_json(p)


class BenchmarkEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.budget = {'status':'REGISTERED', 'approver':'test-only', 'registered_at':1, 'candidate_sha':'a'*40,
                       'limits':[{'profile':'test', 'fixture':'F0', 'metric':'wall', 'unit':'ms', 'maximum':10, 'min_samples':2}]}
        self.result = {'checkout_sha':'a'*40, 'measured_at':2, 'measurements':[
            {'profile':'test','fixture':'F0','metric':'wall','unit':'ms','samples':[1,2]}]}

    def verify(self, digest=None):
        hashed=hashlib.sha256((json.dumps(self.budget, sort_keys=True, separators=(',', ':'))+'\n').encode()).hexdigest()
        validate_benchmarks(self.result,self.budget,digest or hashed)

    def test_valid_numeric_comparison(self): self.verify()

    def test_echo_or_zero_artifacts(self):
        for r in [{}, {'message':'manual review needed'}, {'measurements':[]}]:
            self.result=r
            with self.assertRaises(InvalidEvidence): self.verify()

    def test_regression(self):
        self.result['measurements'][0]['samples']=[11,12]
        with self.assertRaisesRegex(InvalidEvidence,'exceeded'): self.verify()

    def test_nan_bool_negative_and_few_samples(self):
        for samples in [[float('nan'),1],[True,2],[-1,2],[1]]:
            self.result['measurements'][0]['samples']=samples
            with self.assertRaises(InvalidEvidence): self.verify()

    def test_preregistration_identity_unit_and_completeness(self):
        for change in ['hash','time','sha','unit','duplicate']:
            with self.subTest(change=change):
                self.setUp()
                if change=='time': self.result['measured_at']=0
                if change=='sha': self.result['checkout_sha']='f'*40
                if change=='unit': self.result['measurements'][0]['unit']='seconds'
                if change=='duplicate': self.result['measurements']*=2
                with self.assertRaises(InvalidEvidence): self.verify('f'*64 if change=='hash' else None)


if __name__ == '__main__':
    unittest.main()
