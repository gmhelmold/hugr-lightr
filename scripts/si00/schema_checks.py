"""Golden metadata and malformed-input checks; must run with the native hash probe.

This validates the specification oracle, not a future production Rust decoder.
--generate is authoring-only; normal CI consumes checked-in immutable vectors.
"""
from __future__ import annotations
import argparse
import base64
import copy
import hashlib
import json
import struct
import sys
import unittest
from pathlib import Path
from schema_reference import DOMAIN, TAGS, LIMITS, REF_DOMAIN, Invalid, NativeHash, decode, frame, refrecord

HASH = None
VECTORS = []

def b64(raw):
    return base64.b64encode(raw).decode('ascii')

def rr(name='@si00/base', root=b'\x11'*32, parent=None, version='0.1.0'):
    nb=name.encode(); vb=version.encode()
    return struct.pack('<H',len(nb))+nb+root+(b'\0' if parent is None else b'\1'+parent)+struct.pack('<QH',17,len(vb))+vb

def encoded(kind,obj):
    return frame(kind,json.dumps(obj,separators=(',',':'),ensure_ascii=True).encode(),HASH)

def specimens():
    a={'record':b64(rr()),'config':None,'image_manifest':None}
    b={'record':b64(rr(root=b'\x22'*32,parent=b'\x11'*32)),'config':'33'*32,'image_manifest':'44'*32}
    history={'version':1,**b,'provenance':'COMMITTED'}
    old={'version':1,**a,'provenance':'ADOPTED_BASELINE'}
    empty={'record':None,'config':None,'image_manifest':None}
    pending={'version':1,'operation_id':'01'*16,'operation_kind':'IMPORT','ref_name':'@si00/base',
             'ref_key':HASH(REF_DOMAIN+b'@si00/base').hex(),'phase':'PREPARED',
             'before':a,'after':b,'slot':2,'expected_envelope':b64(encoded('history',history)),'retirement':None}
    values=[('ready-unix','ready',{'version':1,'digest':'11'*32,'length':1024,'assurance':'unix-file-directory-v1'}),
            ('ready-windows-u64-max','ready',{'version':1,'digest':'22'*32,'length':2**64-1,'assurance':'windows-file-v1'}),
            ('history-with-oci','history',history),('history-adopted','history',old),
            ('pending-prepared-import','pending',pending)]
    decision=copy.deepcopy(pending);decision['phase']='COMMIT_DECIDED';values.append(('pending-decided-import','pending',decision))
    new=copy.deepcopy(pending);new['before']=empty;values.append(('pending-first-publication','pending',new))
    meta=copy.deepcopy(pending);meta['operation_kind']='METADATA';meta['after']['record']=a['record'];mh={**history,'record':a['record']};meta['expected_envelope']=b64(encoded('history',mh));values.append(('pending-same-root-metadata','pending',meta))
    for phase in ['PREPARED','COMMIT_DECIDED']:
        unt=copy.deepcopy(pending);unt.update(operation_kind='UNTAG',phase=phase,after=empty,slot=None,expected_envelope=None,retirement={'entries':2,'generation_digest':'55'*32})
        values.append(('pending-untag-'+phase.lower(),'pending',unt))
    return values

class SchemaChecks(unittest.TestCase):
    def test_locked_hash_known_empty(self):
        self.assertEqual(HASH(b'').hex(),'af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262')
    def test_pinned_golden_bytes(self):
        self.assertEqual(len(VECTORS),10)
        for v in VECTORS:
            raw=bytes.fromhex(v['wire_hex'])
            self.assertEqual(hashlib.sha256(raw).hexdigest(),v['wire_sha256'])
            self.assertEqual(decode(raw,v['kind'],HASH),v['body'])
            self.assertEqual(encoded(v['kind'],v['body']),raw)
    def test_all_truncation_offsets(self):
        for v in VECTORS:
            raw=bytes.fromhex(v['wire_hex'])
            for i in range(len(raw)):
                with self.subTest(vector=v['id'],offset=i), self.assertRaises(Invalid):decode(raw[:i],v['kind'],HASH)
    def test_header_length_and_checksum(self):
        for v in VECTORS:
            raw=bytes.fromhex(v['wire_hex']); kind=v['kind']
            for bad in [raw+b'\0',bytes([1])+raw[1:],raw[:8]+struct.pack('<I',LIMITS[kind]+1)+raw[12:],raw[:-1]+bytes([raw[-1]^1])]:
                with self.assertRaises(Invalid):decode(bad,kind,HASH)
    def test_duplicate_invalid_utf8_and_nonfinite(self):
        for body in [b'{"version":1,"version":1}',b'"\xff"',b'{"x":NaN}',b'{"x":Infinity}']:
            with self.assertRaises(Invalid):decode(frame('ready',body,HASH),'ready',HASH)
    def test_semantic_negatives_are_resigned(self):
        kind='ready';good=VECTORS[0]['body']
        for key,bad in [('version',True),('version',2),('length',True),('length',1.0),('length',-1),('length',2**64),('digest','FF'*32),('assurance','powerloss-certified')]:
            obj={**good,key:bad}
            with self.subTest(key=key,bad=bad),self.assertRaises(Invalid):decode(encoded(kind,obj),kind,HASH)
        for obj in [{**good,'unknown':0},{k:v for k,v in good.items() if k!='length'}]:
            with self.assertRaises(Invalid):decode(encoded(kind,obj),kind,HASH)
    def test_legacy_full_consumption_and_tag_disjointness(self):
        self.assertEqual(refrecord(rr())['name'],'@si00/base')
        raw=rr()
        for i in range(len(raw)):
            with self.assertRaises(Invalid):refrecord(raw[:i])
        for bad in [raw+b'\0',rr(name=''),rr(name='UPPER'),rr(name='@'+('a'*33)+'/b')]:
            with self.assertRaises(Invalid):refrecord(bad)
        for tag in TAGS.values():
            with self.assertRaises(Invalid):refrecord(tag+b'\0'*100)
    def test_tuple_cross_binding(self):
        v=next(v for v in VECTORS if v['id']=='pending-prepared-import')
        for change in ['key','name','envelope','pointer','orphan','slot','phase']:
            obj=copy.deepcopy(v['body'])
            if change=='key':obj['ref_key']='00'*32
            if change=='name':obj['after']['record']=b64(rr(name='other'))
            if change=='envelope':obj['after']['config']='66'*32
            if change=='pointer':obj['before']['config']='../escape'
            if change=='orphan':obj['before']['record']=None;obj['before']['config']='77'*32
            if change=='slot':obj['slot']=2**64
            if change=='phase':obj['phase']='MAYBE'
            with self.subTest(change=change),self.assertRaises(Invalid):decode(encoded('pending',obj),'pending',HASH)
    def test_base64_and_nested_duplicate(self):
        v=copy.deepcopy(next(v for v in VECTORS if v['kind']=='history')['body'])
        for bad in ['?','QQ===',v['record']+'\n']:
            obj={**v,'record':bad}
            with self.assertRaises(Invalid):decode(encoded('history',obj),'history',HASH)
        p=next(v for v in VECTORS if v['kind']=='pending')['body'];body=json.dumps(p,separators=(',',':')).replace('"config":null','"config":null,"config":null',1).encode()
        with self.assertRaises(Invalid):decode(frame('pending',body,HASH),'pending',HASH)
    def test_ready_body_exact_limit_and_over(self):
        body=json.dumps(VECTORS[0]['body'],separators=(',',':')).encode()
        raw=frame('ready',body+b' '*(4096-len(body)),HASH)
        self.assertEqual(decode(raw,'ready',HASH),VECTORS[0]['body'])
        with self.assertRaises(Invalid):frame('ready',body+b' '*4096,HASH)
    def test_unicode_and_u16_boundary(self):
        self.assertEqual(refrecord(rr(version='\u00e9'*32767))['name'],'@si00/base')
        with self.assertRaises(UnicodeError):rr(version='\ud800')
        with self.assertRaises(struct.error):rr(version='x'*65536)


def main():
    global HASH,VECTORS
    p=argparse.ArgumentParser();p.add_argument('--hash-bin',required=True,type=Path);p.add_argument('--vectors',required=True,type=Path);p.add_argument('--generate',action='store_true');p.add_argument('--result',required=True,type=Path);a=p.parse_args()
    HASH=NativeHash(a.hash_bin)
    if a.generate:
        values=[]
        for name,kind,obj in specimens():
            wire=encoded(kind,obj);values.append({'id':name,'kind':kind,'body':obj,'wire_hex':wire.hex(),'wire_sha256':hashlib.sha256(wire).hexdigest()})
        a.vectors.parent.mkdir(parents=True,exist_ok=True);a.vectors.write_text(json.dumps({'schema':1,'vectors':values},indent=2)+'\n')
    VECTORS=json.loads(a.vectors.read_text())['vectors']
    result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(SchemaChecks))
    a.result.parent.mkdir(parents=True,exist_ok=True)
    a.result.write_text(json.dumps({'schema':1,'tests':result.testsRun,'failures':len(result.failures),'errors':len(result.errors),'skipped':len(result.skipped),'golden_sha256':hashlib.sha256(a.vectors.read_bytes()).hexdigest(),'generated_this_run':a.generate,'runtime_codec_tested':False},indent=2)+'\n')
    return 0 if result.wasSuccessful() and result.testsRun==11 and not result.skipped else 1
if __name__=='__main__':sys.exit(main())
