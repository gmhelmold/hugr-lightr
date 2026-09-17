"""Executable SI00 metadata specification; not a runtime codec or store writer.

Hashing is delegated to the repository-locked native BLAKE3 probe. Golden frames
are immutable inputs on verify runs; generation is an explicit authoring action.
"""
from __future__ import annotations

import base64
import hashlib
import json
import re
import struct
import subprocess
from pathlib import Path

DOMAIN = b'lightr/snapshot-integrity/metadata/v1/'
TAGS = {'ready': b'\0\0LSIR01', 'history': b'\0\0LSIH01', 'pending': b'\0\0LSIP01'}
LIMITS = {'ready': 4096, 'history': 196608, 'pending': 1048576}
REF_DOMAIN = b'lightr/ref/v1/'
HEX = re.compile(r'[0-9a-f]{64}\Z')
NAME = re.compile(r'(?:@[a-z0-9-]{1,32}/)?[a-z0-9._-]{1,64}\Z')
MAX_U64 = 2**64 - 1

class Invalid(ValueError):
    pass

def need(ok: bool, message: str) -> None:
    if not ok:
        raise Invalid(message)

def fields(value, expected):
    need(type(value) is dict and set(value) == set(expected), 'unexpected/missing fields')

def uint(value):
    need(type(value) is int and 0 <= value <= MAX_U64, 'invalid u64')

def digest(value):
    need(type(value) is str and HEX.fullmatch(value) is not None, 'invalid digest')

def unbase(value, limit):
    need(type(value) is str and len(value) <= 4*((limit+2)//3), 'base64 exceeds bound')
    try:
        raw = base64.b64decode(value, validate=True)
    except (ValueError, UnicodeError) as exc:
        raise Invalid('invalid base64') from exc
    need(len(raw) <= limit and base64.b64encode(raw).decode() == value, 'noncanonical base64')
    return raw

def refrecord(raw):
    need(len(raw) <= 131147, 'overlong RefRecord')
    pos = 0
    def take(size):
        nonlocal pos
        need(pos + size <= len(raw), 'truncated RefRecord')
        result = raw[pos:pos+size]; pos += size
        return result
    try:
        name = take(int.from_bytes(take(2), 'little')).decode('utf-8')
        need(NAME.fullmatch(name) is not None, 'invalid ref name')
        root = take(32).hex()
        parent = take(1)[0]
        need(parent in (0, 1), 'invalid parent tag')
        if parent:
            take(32)
        take(8)
        take(int.from_bytes(take(2), 'little')).decode('utf-8')
        need(pos == len(raw), 'trailing RefRecord bytes')
        return {'name': name, 'root': root}
    except UnicodeError as exc:
        raise Invalid('invalid RefRecord UTF-8') from exc

def named(value):
    fields(value, ('record', 'config', 'image_manifest'))
    ref = None if value['record'] is None else refrecord(unbase(value['record'], 131147))
    for key in ('config', 'image_manifest'):
        if value[key] is not None:
            digest(value[key])
    need(ref is not None or (value['config'] is None and value['image_manifest'] is None), 'orphan named pointers')
    return ref

class NativeHash:
    def __init__(self, binary: Path):
        self.binary = binary.resolve()
        self.cache = {}
    def __call__(self, raw: bytes) -> bytes:
        key = hashlib.sha256(raw).digest()
        if key not in self.cache:
            proc = subprocess.run([str(self.binary)], input=raw.hex().encode()+b'\n', capture_output=True, timeout=15)
            need(proc.returncode == 0, 'native hash probe failed')
            text = proc.stdout.decode('ascii').strip()
            digest(text); self.cache[key] = bytes.fromhex(text)
        return self.cache[key]

def body_check(kind, obj, hashfn):
    if kind == 'ready':
        fields(obj, ('version', 'digest', 'length', 'assurance'))
        digest(obj['digest']); uint(obj['length'])
        need(obj['assurance'] in ('unix-file-directory-v1', 'windows-file-v1'), 'unknown assurance')
    elif kind == 'history':
        fields(obj, ('version', 'record', 'config', 'image_manifest', 'provenance'))
        ref = named({k: obj[k] for k in ('record', 'config', 'image_manifest')})
        need(ref is not None, 'history without record')
        need(obj['provenance'] in ('COMMITTED', 'ADOPTED_BASELINE'), 'invalid provenance')
    else:
        fields(obj, ('version', 'operation_id', 'operation_kind', 'ref_name', 'ref_key',
                     'phase', 'before', 'after', 'slot', 'expected_envelope', 'retirement'))
        need(type(obj['operation_id']) is str and re.fullmatch(r'[0-9a-f]{32}', obj['operation_id']), 'invalid operation id')
        need(obj['operation_kind'] in ('SNAPSHOT','BUILD','IMPORT','PULL','LOAD','TAG','UNDO','UNTAG','ADOPT','METADATA'), 'unknown operation')
        need(type(obj['ref_name']) is str and NAME.fullmatch(obj['ref_name']), 'invalid ref name')
        digest(obj['ref_key'])
        need(hashfn(REF_DOMAIN + obj['ref_name'].encode()).hex() == obj['ref_key'], 'ref-key mismatch')
        need(obj['phase'] in ('PREPARED','COMMIT_DECIDED'), 'unknown phase')
        refs = [named(obj[k]) for k in ('before', 'after')]
        need(all(r is None or r['name'] == obj['ref_name'] for r in refs), 'tuple name mismatch')
        if obj['operation_kind'] in ('UNDO', 'ADOPT'):
            need(refs[0] is not None, 'operation requires existing reference')
        if obj['operation_kind'] == 'UNTAG':
            need(refs[0] is not None and refs[1] is None, 'invalid untag transition')
            need(obj['slot'] is None and obj['expected_envelope'] is None, 'untag appends history')
            fields(obj['retirement'], ('entries', 'generation_digest'))
            uint(obj['retirement']['entries']); digest(obj['retirement']['generation_digest'])
        else:
            need(refs[1] is not None and obj['retirement'] is None, 'invalid publishing transition')
            uint(obj['slot'])
            frame_bytes = unbase(obj['expected_envelope'], LIMITS['history']+44)
            hist = decode(frame_bytes, 'history', hashfn)
            need(all(hist[k] == obj['after'][k] for k in ('record','config','image_manifest')), 'history tuple mismatch')
            need(hist['provenance'] == ('ADOPTED_BASELINE' if obj['operation_kind'] == 'ADOPT' else 'COMMITTED'), 'provenance/operation mismatch')
    need(type(obj['version']) is int and obj['version'] == 1, 'unsupported version')

def decode(raw: bytes, kind: str, hashfn):
    need(kind in TAGS, 'unknown record type')
    need(len(raw) >= 44 and raw[:8] == TAGS[kind], 'truncated/bad header')
    size = struct.unpack('<I', raw[8:12])[0]
    need(size <= LIMITS[kind], 'declared length exceeds bound')
    need(len(raw) == 44 + size, 'length/trailing bytes')
    need(hashfn(DOMAIN + raw[:-32]) == raw[-32:], 'checksum mismatch')
    def unique(pairs):
        value = {}
        for k, v in pairs:
            need(k not in value, 'duplicate JSON field')
            value[k] = v
        return value
    def no_constant(_):
        raise Invalid('nonfinite JSON number')
    try:
        value = json.loads(raw[12:-32].decode('utf-8'), object_pairs_hook=unique, parse_constant=no_constant)
    except (UnicodeError, ValueError, RecursionError) as exc:
        raise Invalid('invalid JSON body: '+str(exc)) from exc
    body_check(kind, value, hashfn)
    return value

def frame(kind, raw_body, hashfn):
    need(len(raw_body) <= LIMITS[kind], 'body exceeds bound')
    prefix = TAGS[kind]+struct.pack('<I', len(raw_body))+raw_body
    return prefix+hashfn(DOMAIN+prefix)
