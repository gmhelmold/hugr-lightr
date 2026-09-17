"""Exercise ENOSPC on an owned 32 MiB loop image, never fill the host disk.

This proves that the runner can host a constrained-resource experiment. It is
NOT a test of the future Lightr journal or resource-recovery implementation.
"""
from __future__ import annotations

import argparse
import errno
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path


def main() -> None:
    parser=argparse.ArgumentParser(); parser.add_argument('--out', type=Path, required=True)
    out=parser.parse_args().out.resolve(); out.parent.mkdir(parents=True, exist_ok=True)
    work=Path(tempfile.mkdtemp(prefix='si00-loop-', dir=os.environ.get('RUNNER_TEMP')))
    image=work/'owned.img'; mount=work/'mount'; mount.mkdir()
    result={'stage':'SI00_OS_RESOURCE_CAPABILITY', 'status':'STARTED', 'image_bytes':32*1024*1024,
            'run_id':os.environ.get('GITHUB_RUN_ID'), 'runtime_recovery_tested':False}
    mounted=False
    def run(argv): subprocess.run(argv, check=True, capture_output=True, timeout=30)
    try:
        if shutil.disk_usage(work).free < 128*1024*1024: raise RuntimeError('Insufficient safe host headroom')
        with image.open('xb') as stream: stream.truncate(result['image_bytes'])
        run(['mkfs.ext4','-q','-F','-m','0',str(image)])
        run(['sudo','-n','mount','-o','loop,nosuid,nodev,noexec',str(image),str(mount)]); mounted=True
        run(['sudo','-n','chown',f'{os.getuid()}:{os.getgid()}',str(mount)])
        retained=mount/'retained'; retained.write_bytes(b'SI00-retained\n')
        with retained.open('r+b') as stream: os.fsync(stream.fileno())
        observed=None
        try:
            with (mount/'filler').open('xb', buffering=0) as stream:
                for _ in range(64):
                    stream.write(b'x'*(1024*1024)); os.fsync(stream.fileno())
        except OSError as exc:
            observed=exc.errno
        if observed != errno.ENOSPC: raise RuntimeError(f'Expected ENOSPC, observed {observed}')
        if retained.read_bytes()!=b'SI00-retained\n': raise RuntimeError('Retained data changed')
        (mount/'filler').unlink()
        retry=mount/'retry'; retry.write_bytes(b'resources-restored\n')
        with retry.open('r+b') as stream: os.fsync(stream.fileno())
        if retry.read_bytes()!=b'resources-restored\n': raise RuntimeError('Retry mismatch')
        result.update(status='EXERCISED', errno=observed, intervention='remove only owned filler', retained_bytes_verified=True)
    except Exception as exc:
        result.update(status='FAILED', error=f'{type(exc).__name__}: {exc}')
        raise
    finally:
        try:
            if mounted: run(['sudo','-n','umount',str(mount)]); mounted=False
        finally:
            result['unmounted']=not mounted
            out.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
            if not mounted: shutil.rmtree(work)
            # On unmount failure leave the isolated mount untouched, fail job.


if __name__=='__main__': main()
