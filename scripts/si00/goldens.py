"""Read immutable native-authored golden bytes; storage compression is not generation."""
from __future__ import annotations
import gzip
import hashlib
from pathlib import Path

GOLDEN_SHA = '8e0a4b7c52f99e12a8ecf91a4bdc706b547f5524d8e7464d06972e0b5b3221df'
MAX_BYTES = 24177

def golden_bytes(path: Path) -> bytes:
    """Bound both the stored input and decoded bytes before JSON parsing."""
    if path.stat().st_size > MAX_BYTES:
        raise ValueError('golden input exceeds bound')
    opener = gzip.open if path.suffix == '.gz' else open
    with opener(path, 'rb') as stream:
        raw = stream.read(MAX_BYTES + 1)
    if len(raw) != MAX_BYTES or hashlib.sha256(raw).hexdigest() != GOLDEN_SHA:
        raise ValueError('immutable golden bytes changed')
    return raw
