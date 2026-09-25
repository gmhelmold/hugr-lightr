"""Independent-hash checks of the canonical SI00 oracle, never a runtime codec.

The fixed vectors were produced by the repository's native locked BLAKE3 probe
in run 35237459468. These tests do not regenerate them or execute that binary.
"""
from __future__ import annotations
import base64
import copy
import hashlib
import json
from pathlib import Path
import struct
import unittest
from unittest.mock import patch

import schema_reference as s
from vendor.pure_blake3 import Hasher
from goldens import golden_bytes

ROOT=Path(__file__).resolve().parents[2]
VECTORS=ROOT/'docs/plans/snapshot-integrity/bootstrap/vectors/schema-vectors.json.gz'
GOLDEN_SHA='8e0a4b7c52f99e12a8ecf91a4bdc706b547f5524d8e7464d06972e0b5b3221df'


def hash_bytes(data: bytes) -> bytes:
    h=Hasher();h.update(data);return bytes(h.finalize())


def encode(kind: str, obj: dict) -> bytes:
    return s.frame(kind,json.dumps(obj,separators=(',',':'),ensure_ascii=True,allow_nan=False).encode(),hash_bytes)


def raw_record(name='@si00/base',tool='0.1.0',parent=None,created=17):
    nb=name.encode();tb=tool.encode()
    return struct.pack('<H',len(nb))+nb+b'\x11'*32+(b'\0' if parent is None else b'\1'+parent)+struct.pack('<QH',created,len(tb))+tb


class SchemaConformanceTests(unittest.TestCase):
    def setUp(self):
        self.rows=json.loads(golden_bytes(VECTORS).decode('utf-8'))['vectors']
        self.ready=self.rows[0]['body']
        self.pending=next(x['body'] for x in self.rows if x['id']=='pending-prepared-import')

    def test_upstream_reference_and_nine_official_vectors(self):
        raw=(ROOT/'scripts/si00/vendor/pure_blake3.py').read_bytes()
        self.assertEqual(hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest(),'5a9db753540be51c0d3307656299c02e2900eb6c')
        expected={
            0:'af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262',
            1:'2d3adedff11b61f14c886e35afa036736dcd87a74d27b5c1510225d0f592e213',
            63:'e9bc37a594daad83be9470df7f7b3798297c3d834ce80ba85d6e207627b7db7b',
            64:'4eed7141ea4a5cd4b788606bd23f46e212af9cacebacdc7d1f4c6dc7f2511b98',
            65:'de1e5fa0be70df6d2be8fffd0e99ceaa8eb6e8c93a63f2d8d1c30ecb6b263dee',
            1023:'10108970eeda3eb932baac1428c7a2163b0e924c9a9e25b35bba72b28f70bd11',
            1024:'42214739f095a406f3fc83deb889744ac00df831c10daa55189b5d121c855af7',
            1025:'d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444',
            2048:'e776b6028c7cd22a4d0ba182a8bf62205d2ef576467e838ed6f2529b85fba24a'}
        for length,want in expected.items():
            with self.subTest(length=length):
                data=bytes(i%251 for i in range(length));self.assertEqual(hash_bytes(data).hex(),want)
                h=Hasher()
                for n in range(0,len(data),37):h.update(data[n:n+37])
                self.assertEqual(bytes(h.finalize()).hex(),want)

    def test_immutable_native_goldens_match_independent_hash(self):
        self.assertEqual(hashlib.sha256(golden_bytes(VECTORS)).hexdigest(),GOLDEN_SHA)
        self.assertEqual(len(self.rows),10)
        self.assertEqual(len({r['id'] for r in self.rows}),10)
        for row in self.rows:
            with self.subTest(vector=row['id']):
                raw=bytes.fromhex(row['wire_hex'])
                self.assertEqual(hashlib.sha256(raw).hexdigest(),row['wire_sha256'])
                self.assertEqual(encode(row['kind'],row['body']),raw)
                self.assertEqual(s.decode(raw,row['kind'],hash_bytes),row['body'])

    def test_every_strict_golden_prefix_rejected(self):
        count=0
        for row in self.rows:
            raw=bytes.fromhex(row['wire_hex'])
            for size in range(len(raw)):
                with self.subTest(vector=row['id'],size=size),self.assertRaises(s.Invalid):
                    s.decode(raw[:size],row['kind'],hash_bytes)
                count+=1
        self.assertEqual(count,7020)

    def test_trailing_wrong_tag_and_checksum(self):
        for row in self.rows:
            raw=bytes.fromhex(row['wire_hex'])
            for bad in [raw+b'\0',b'\xff'+raw[1:],raw[:-1]+bytes([raw[-1]^1]),raw[:12]+bytes([raw[12]^1])+raw[13:]]:
                with self.assertRaises(s.Invalid):s.decode(bad,row['kind'],hash_bytes)

    def test_resigned_duplicate_unknown_and_missing_fields(self):
        for body in [b'{"version":1,"version":1}',b'{"version":1}',json.dumps(dict(self.ready,extra=1)).encode(),b'{"version":1,"x":{"a":0,"a":1}}']:
            with self.assertRaises(s.Invalid):s.decode(s.frame('ready',body,hash_bytes),'ready',hash_bytes)

    def test_invalid_json_unicode_numbers_and_schema_version(self):
        for body in [b'\xff',b'{',b'[]',b'null',b'"text"',b'NaN',b'Infinity']:
            with self.assertRaises(s.Invalid):s.decode(s.frame('ready',body,hash_bytes),'ready',hash_bytes)
        for field in ('version','length'):
            for value in (True,1.0,'1',-1,s.MAX_U64+1):
                obj=dict(self.ready);obj[field]=value
                with self.assertRaises(s.Invalid):s.decode(encode('ready',obj),'ready',hash_bytes)
        for change in [dict(version=2),dict(assurance='POWER_LOSS_PROVEN')]:
            with self.assertRaises(s.Invalid):s.decode(encode('ready',dict(self.ready,**change)),'ready',hash_bytes)

    def test_declared_bound_checked_before_hashing_body(self):
        for kind,limit in s.LIMITS.items():
            for length in [limit+1,2**32-1]:
                raw=s.TAGS[kind]+struct.pack('<I',length)+b'\0'*32
                def forbidden(_):raise AssertionError('hash attempted before length rejection')
                with self.assertRaisesRegex(s.Invalid,'bound'):s.decode(raw,kind,forbidden)

    def test_exact_maximum_frame_bodies_and_one_over(self):
        for kind in s.TAGS:
            obj=next(r['body'] for r in self.rows if r['kind']==kind)
            body=json.dumps(obj,separators=(',',':')).encode();body+=b' '*(s.LIMITS[kind]-len(body))
            self.assertEqual(s.decode(s.frame(kind,body,hash_bytes),kind,hash_bytes),obj)
            with self.assertRaises(s.Invalid):s.frame(kind,body+b' ',hash_bytes)

    def test_legacy_lengths_parent_flags_and_full_consumption(self):
        name='@'+'a'*32+'/'+'b'*64
        raw=raw_record(name,tool='x'*65535,parent=b'\x22'*32,created=s.MAX_U64)
        self.assertEqual(s.refrecord(raw)['name'],name)
        with self.assertRaises(struct.error):raw_record(tool='x'*65536)
        with self.assertRaises(s.Invalid):s.refrecord(raw+b'\0')
        small=raw_record()
        for n in range(len(small)):
            with self.assertRaises(s.Invalid):s.refrecord(small[:n])
        bad=bytearray(small);bad[2+len('@si00/base')+32]=2
        with self.assertRaises(s.Invalid):s.refrecord(bytes(bad))
        for name in ['', '@a/','@'+'a'*33+'/b','A','a/b','x'*65]:
            with self.assertRaises(s.Invalid):s.refrecord(raw_record(name))

    def test_reserved_tags_cannot_decode_as_legacy(self):
        for raw in [b'\0\0LSIH99'+b'\0'*100,b'\0\0',bytes.fromhex(self.rows[0]['wire_hex'])]:
            with self.assertRaises(s.Invalid):s.refrecord(raw)

    def test_base64_and_hex_canonicality(self):
        for text in ['AA','AA==\n','AB==','%%%%','AAAA====']:
            with self.assertRaises(s.Invalid):s.unbase(text,16)
        for text in ['A'*64,'0'*63,'0'*65,'x'*64,'../'+'0'*61]:
            with self.assertRaises(s.Invalid):s.digest(text)

    def test_pending_name_key_phase_id_and_slot_consistency(self):
        for key,value in [('ref_key','00'*32),('phase','UNKNOWN'),('operation_kind','GENERIC'),('operation_id','abc'),('slot',True)]:
            obj=copy.deepcopy(self.pending);obj[key]=value
            with self.assertRaises(s.Invalid):s.decode(encode('pending',obj),'pending',hash_bytes)
        obj=copy.deepcopy(self.pending);obj['after']['record']=base64.b64encode(raw_record('other')).decode()
        with self.assertRaisesRegex(s.Invalid,'name'):s.decode(encode('pending',obj),'pending',hash_bytes)

    def test_envelope_binds_every_tuple_component(self):
        for key,value in [('config','ab'*32),('image_manifest','cd'*32),('record',base64.b64encode(raw_record()).decode())]:
            obj=copy.deepcopy(self.pending);obj['after'][key]=value
            with self.assertRaisesRegex(s.Invalid,'history tuple'):s.decode(encode('pending',obj),'pending',hash_bytes)

    def test_explicit_nulls_and_untag_retirement_shape(self):
        obj=copy.deepcopy(self.pending);del obj['after']['config']
        with self.assertRaises(s.Invalid):s.decode(encode('pending',obj),'pending',hash_bytes)
        unt=copy.deepcopy(next(r['body'] for r in self.rows if r['id']=='pending-untag-prepared'))
        for key,value in [('slot',1),('expected_envelope',self.pending['expected_envelope']),('retirement',None)]:
            obj=copy.deepcopy(unt);obj[key]=value
            with self.assertRaises(s.Invalid):s.decode(encode('pending',obj),'pending',hash_bytes)
        for key,value in [('entries',True),('generation_digest','x'*64)]:
            obj=copy.deepcopy(unt);obj['retirement'][key]=value
            with self.assertRaises(s.Invalid):s.decode(encode('pending',obj),'pending',hash_bytes)

    def test_undo_and_adoption_require_a_previous_reference(self):
        for kind in ['UNDO','ADOPT']:
            obj=copy.deepcopy(self.pending);obj['operation_kind']=kind
            obj['before']={'record':None,'config':None,'image_manifest':None}
            if kind=='ADOPT':
                hist=s.decode(base64.b64decode(obj['expected_envelope']),'history',hash_bytes)
                hist['provenance']='ADOPTED_BASELINE';obj['expected_envelope']=base64.b64encode(encode('history',hist)).decode()
            with self.subTest(kind=kind),self.assertRaisesRegex(s.Invalid,'existing reference'):
                s.decode(encode('pending',obj),'pending',hash_bytes)

    def test_metadata_only_change_keeps_same_record_but_different_tuple(self):
        obj=next(r['body'] for r in self.rows if r['id']=='pending-same-root-metadata')
        self.assertEqual(obj['before']['record'],obj['after']['record'])
        self.assertNotEqual(obj['before'],obj['after'])
        self.assertEqual(s.decode(encode('pending',obj),'pending',hash_bytes),obj)

    def test_read_does_not_replace_original_legal_frame_bytes(self):
        body=json.dumps(self.ready,indent=2).encode();raw=s.frame('ready',body,hash_bytes)
        obj=s.decode(raw,'ready',hash_bytes)
        self.assertEqual(obj,self.ready);self.assertNotEqual(raw,encode('ready',obj))
        self.assertEqual(raw,s.frame('ready',body,hash_bytes))

    def test_checksum_control_is_causally_exercised(self):
        good=bytes.fromhex(self.rows[0]['wire_hex']);bad=good[:-1]+bytes([good[-1]^1])
        with self.assertRaisesRegex(s.Invalid,'checksum'):s.decode(bad,'ready',hash_bytes)
        self.assertEqual(s.decode(bad,'ready',lambda _:bad[-32:]),self.ready)


if __name__=='__main__':unittest.main()
