"""Bind the Rust readiness fixtures to the already accepted SI-00 wire bytes."""
from pathlib import Path
import gzip
import hashlib
import json

ROOT = Path(__file__).resolve().parents[2]
source = gzip.decompress((ROOT / 'docs/plans/snapshot-integrity/bootstrap/vectors/schema-vectors.json.gz').read_bytes())
assert hashlib.sha256(source).hexdigest() == '8e0a4b7c52f99e12a8ecf91a4bdc706b547f5524d8e7464d06972e0b5b3221df'
expected = {v['id']: bytes.fromhex(v['wire_hex']) for v in json.loads(source)['vectors'] if v['kind'] == 'ready'}
actual = {p.stem: bytes.fromhex(p.read_text()) for p in (ROOT / 'crates/lightr-store/src/store/cas/foundation/vectors').glob('*.hex')}
assert len(expected) == 2 and actual == expected, 'Rust vectors differ from frozen SI-00 expectations'
print('Frozen readiness vectors verified; no generation')
