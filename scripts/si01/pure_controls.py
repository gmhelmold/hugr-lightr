"""Compiled negative controls for the pure SI-01 slice only.

No Store is opened. Mutations are confined to a temporary source archive; each
must compile then fail one exact named Rust test. A timeout is never a kill.
"""
from __future__ import annotations
import argparse
import gzip
import hashlib
import io
import json
import os
import signal
from pathlib import Path
import re
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]
PREFIX = 'store::foundation::ready_tests::'
CASES = [
    ('checksum', 'crates/lightr-store/src/store/foundation/ready.rs',
     'if Digest::of_bytes(&input).0.as_slice() != &tail[length..] {',
     'if false {', 'ready_matches_frozen_si00_golden_bytes'),
    ('length-before-read', 'crates/lightr-store/src/store/foundation/ready.rs',
     'if length > LIMIT {', 'if length > u32::MAX as usize {',
     'oversized_header_is_rejected_before_reading_body'),
    ('unknown-fields', 'crates/lightr-store/src/store/foundation/ready.rs',
     '#[serde(deny_unknown_fields)]', '', 'readiness_rejects_resigned_semantic_corruption'),
    ('error-payload', 'crates/lightr-store/src/store/foundation/error.rs',
     'LightrError::Io(io::Error::new(self.cause.kind(), self))',
     'LightrError::Io(io::Error::new(self.cause.kind(), self.cause))',
     'structured_failure_survives_legacy_wrapper_and_source_chain'),
]


def main() -> int:
    parser=argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, required=True)
    args=parser.parse_args()
    out=args.out.resolve();out.mkdir(parents=True,exist_ok=False)
    head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    tree=subprocess.check_output(['git','rev-parse','HEAD^{tree}'],cwd=ROOT,text=True).strip()
    rows=[]
    receipt={'schema':1,'checkout_sha':head,'tree':tree,'runtime_qualified':False,'controls':rows}
    try:
        canonical=json.loads(gzip.decompress((ROOT/'docs/plans/snapshot-integrity/bootstrap/vectors/schema-vectors.json.gz').read_bytes()))
        wanted=[row for row in canonical['vectors'] if row['kind']=='ready']
        actual=json.loads((ROOT/'crates/lightr-store/src/store/foundation/ready-golden.json').read_text())
        if actual!=wanted:raise ValueError('readiness vectors diverged from SI00 immutable authority')
        for row in actual:
            if hashlib.sha256(bytes.fromhex(row['wire_hex'])).hexdigest()!=row['wire_sha256']:
                raise ValueError('bad vector checksum')
        archive=subprocess.check_output(['git','archive','--format=zip','HEAD'],cwd=ROOT)
        (out/'source.zip').write_bytes(archive)
        def run(argv,cwd,name,limit=600):
            with (out/(name+'.log')).open('wb') as stream:
                process=subprocess.Popen(argv,cwd=cwd,stdout=stream,stderr=subprocess.STDOUT,start_new_session=True)
                try:
                    process.wait(timeout=limit)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=30)
                    raise RuntimeError('control timeout is not a causal test failure: '+name)
            return process.returncode,(out/(name+'.log')).read_text(errors='replace')
        with tempfile.TemporaryDirectory(prefix='si01-pure-') as directory:
            root=Path(directory)
            with zipfile.ZipFile(io.BytesIO(archive)) as source:
                for item in source.infolist():
                    if not (root/item.filename).resolve().is_relative_to(root):raise ValueError('unsafe source path')
                source.extractall(root)
            code,text=run(['cargo','+1.96.0','test','--locked','-p','lightr-store','--lib',PREFIX,'--','--test-threads=1'],root,'pristine')
            if code or '5 passed; 0 failed' not in text:raise ValueError('pristine pure suite failed/empty')
            for label,path,old,new,test in CASES:
                file=root/path;original=file.read_text()
                if original.count(old)!=1:raise ValueError('mutation seam drift: '+label)
                file.write_text(original.replace(old,new,1))
                try:
                    code,_=run(['cargo','+1.96.0','test','--locked','-p','lightr-store','--lib','--no-run'],root,label+'-build')
                    if code:raise ValueError('mutant did not compile: '+label)
                    code,text=run(['cargo','+1.96.0','test','--locked','-p','lightr-store','--lib',PREFIX+test,'--','--exact'],root,label+'-test')
                    # Exactly one failing test; unrelated names are intentionally filtered.
                    exact=r'test result: FAILED\. 0 passed; 1 failed; 0 ignored; 0 measured; \d+ filtered out'
                    if code!=101 or not re.search(exact,text) or ('test '+PREFIX+test+' ... FAILED') not in text:
                        raise ValueError('not a causal named-test failure: '+label)
                    rows.append({'id':label,'test':PREFIX+test,'status':'KILLED_CAUSALLY','build_exit':0,'test_exit':code})
                finally:
                    file.write_text(original)
            code,text=run(['cargo','+1.96.0','test','--locked','-p','lightr-store','--lib',PREFIX,'--','--test-threads=1'],root,'restored')
            if code or '5 passed; 0 failed' not in text:raise ValueError('restored suite failed')
        receipt['status']='PURE_SLICE_CONTROLS_PASSED'
    except Exception as error:
        receipt['status']='FAILED';receipt['error']=str(error)
    receipt['artifacts']=[{'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(out.iterdir()) if p.is_file()]
    (out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(receipt))
    return 0 if receipt['status']=='PURE_SLICE_CONTROLS_PASSED' else 1

if __name__=='__main__':raise SystemExit(main())
