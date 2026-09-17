"""Measured pre-remediation CLI baseline; never a performance acceptance verdict.

F0-F5 are measured with plan-prescribed repetitions on an isolated runner. F6
requires SI-03's new history semantics, so this script explicitly does not claim
that the legacy binary is a correct F6 reference. Fixture creation and byte
comparison are outside the command timing, all samples and failures are kept.
"""
from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from evidence import require, sha256
from fixtures import create, identity, tree
from inventory import fingerprint
from probes import filesystem

ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve(); out = args.out.resolve(); out.mkdir(parents=True, exist_ok=False)
    result = {'schema':1, 'kind':'pre-remediation-baseline-not-acceptance', 'status':'STARTED',
              'checkout_sha':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
              'input_fingerprint':fingerprint(ROOT), 'binary_sha256':sha256(binary),
              'run_id':os.environ.get('GITHUB_RUN_ID'), 'machine':platform.machine(), 'os':platform.platform(),
              'started_at':time.time(), 'cpu_count':os.cpu_count(), 'rayon_threads':4,
              'rustc':subprocess.check_output(['rustc','+1.96.0','-Vv'],text=True).strip(),
              'filesystem':filesystem(out), 'fixtures':[], 'budget_status':'NOT_REGISTERED',
              'runtime_qualified':False, 'F6':'BLOCKED_REFERENCE_REQUIRES_SI03',
              'cache_control':'new store versus warmed store; OS page cache not forced cold',
              'method':'F0-F3/F5:3 warmup +20 samples; F4:1 warmup +7 samples; no tail percentile claim'}
    commands=[]
    def command(argv: list[str], home: Path, label: str) -> dict:
        env=dict(os.environ, LIGHTR_HOME=str(home), RAYON_NUM_THREADS='4'); started=time.perf_counter_ns()
        timefile=out/(label+'.resources.json')
        launch=['/usr/bin/time','-f','{"max_rss_kib":%M,"fs_inputs":%I,"fs_outputs":%O}', '-o',str(timefile),str(binary),'--json',*argv]
        p=subprocess.run(launch,cwd=ROOT,env=env,capture_output=True,timeout=120)
        elapsed=(time.perf_counter_ns()-started)/1_000_000
        (out/(label+'.stdout')).write_bytes(p.stdout); (out/(label+'.stderr')).write_bytes(p.stderr)
        row={'argv':[str(binary),'--json',*argv], 'exit':p.returncode,'wall_ms':elapsed,'log':label}
        if p.returncode == 0:
            row['resources']=json.loads(timefile.read_text())
            try: row['report']=json.loads(p.stdout)
            except ValueError: row['invalid_json']=True
        commands.append(row)
        require(p.returncode == 0 and not row.get('invalid_json'), 'CLI baseline failed: '+label)
        return row
    failed=False
    try:
        require(shutil.disk_usage(out).free > 2*1024**3,'need 2 GiB disposable headroom')
        with tempfile.TemporaryDirectory(prefix='si00-cli-',dir=out) as tmp:
            root=Path(tmp)
            for fixture in ['F0','F1','F2','F3','F4','F5']:
                source=root/'source'; create(source,fixture)
                expected=tree(source)
                row={'id':fixture,'generator':'fixtures.py/v1','tree_sha256':identity(expected),
                     'entries':len(expected),'logical_file_bytes':sum(e.get('length',0) for e in expected),
                     'warmups':1 if fixture=='F4' else 3,'measured_count':7 if fixture=='F4' else 20,
                     'samples':[], 'warmup_samples':[]}
                result['fixtures'].append(row)
                (out/(fixture+'-tree.json')).write_text(json.dumps(expected,indent=2)+'\n')
                for i in range(row['warmups']+row['measured_count']):
                    home=root/'home'; home.mkdir()
                    dest=root/'restored'; label=f'{fixture}-{i:02}'
                    cold=command(['snapshot','--dir',str(source),'--name','@si00/base'],home,label+'-new')
                    warm=command(['snapshot','--dir',str(source),'--name','@si00/base'],home,label+'-warm')
                    hydrated=command(['hydrate',str(dest),'--name','@si00/base','--verify'],home,label+'-hydrate')
                    require(tree(dest)==expected,'independent restored-tree mismatch: '+label)
                    record={'new_store_ms':cold['wall_ms'],'warm_store_ms':warm['wall_ms'],
                            'hydrate_ms':hydrated['wall_ms'],'hydrate_rung':hydrated['report'].get('rung'),
                            'new_store_resources':cold['resources'], 'warm_store_resources':warm['resources'],
                            'hydrate_resources':hydrated['resources'], 'new_objects':cold['report'].get('objects_new'),
                            'tree_matches':True}
                    row['warmup_samples' if i<row['warmups'] else 'samples'].append(record)
                    shutil.rmtree(dest); shutil.rmtree(home)
                shutil.rmtree(source)
                print(fixture, 'measured',len(row['samples']), 'tree',row['tree_sha256'],flush=True)
    except Exception as exc:
        result['error']=f'{type(exc).__name__}: {exc}'; failed=True
    finally:
        result['status']='BASELINE_FAILED' if failed else 'BASELINE_MEASURED'
        result['commands']=commands; result['finished_at']=time.time()
        (out/'baseline.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    return int(failed)


if __name__=='__main__': sys.exit(main())
