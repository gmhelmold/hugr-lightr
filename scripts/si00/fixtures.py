"""Deterministic owned fixtures and an independent no-follow tree comparator."""
from __future__ import annotations

import hashlib
import json
import os
import stat
from pathlib import Path


def write_blocks(path: Path, block: bytes, total: int) -> None:
    with path.open('xb') as stream:
        while total:
            piece = block[:min(total, len(block))]
            stream.write(piece); total -= len(piece)


def create(root: Path, fixture: str) -> None:
    root.mkdir(exist_ok=False)
    if fixture == 'F0':
        return
    if fixture == 'F1':
        # Exact layout/bytes of tests/common::fixture_tree at the frozen baseline.
        dirs = ['level1/sub1/deep1', 'level1/sub1/deep2', 'level1/sub2/deep1',
                'level2/sub1/deep1', 'level2/sub2/deep1', 'level3/sub1/deep1']
        for name in dirs: (root/name).mkdir(parents=True)
        (root/'empty_dir').mkdir()
        for i in range(200): (root/dirs[i % len(dirs)]/f'file_{i:04}.txt').write_bytes(b'x'*1024)
        (root/'exec_script.sh').write_bytes(b'#!/bin/sh\necho hello\n')
        if os.name == 'posix':
            (root/'exec_script.sh').chmod(0o755)
            os.symlink('file_0000.txt', root/dirs[0]/'symlink_to_first.txt')
        write_blocks(root/'bigfile.bin', b'\xab'*(1024*1024), 8*1024*1024+1024)
    elif fixture in ('F2', 'F3'):
        for i in range(10000):
            block = hashlib.sha256(f'si00-v1:{i}'.encode()).digest()*128 if fixture == 'F2' else b'x'*1024
            (root/f'file-{i:05}.bin').write_bytes(block)
    elif fixture == 'F4':
        write_blocks(root/'large.bin', bytes(range(256))*4096, 256*1024*1024)
    elif fixture == 'F5':
        (root/'empty').mkdir(); (root/'dir').mkdir()
        (root/'dir'/'payload').write_bytes(b'owned-fixture\x00\xff\n')
        (root/'unicode-caf\u00e9').write_bytes(b'utf8-name\n')
        (root/'executable').write_bytes(b'#!/bin/sh\nexit 0\n')
        if os.name == 'posix':
            (root/'executable').chmod(0o755)
            (root/'literal\\backslash').write_bytes(b'literal-backslash\n')
            os.symlink('dir/payload', root/'file-link')
            os.symlink('absent', root/'dangling-link')
    else:
        raise ValueError('Unknown/non-filesystem fixture: ' + fixture)


def tree(root: Path) -> list[dict]:
    """Uses lstat/scandir/readlink and file bytes, not LMF1/index implementation."""
    result = []
    def walk(directory: Path, prefix: str = '') -> None:
        for entry in sorted(os.scandir(directory), key=lambda e: e.name):
            name = prefix + entry.name; info = entry.stat(follow_symlinks=False)
            row = {'path': name}
            if stat.S_ISLNK(info.st_mode):
                row.update(kind='symlink', target=os.readlink(entry.path))
            elif stat.S_ISDIR(info.st_mode):
                row['kind'] = 'directory'; walk(Path(entry.path), name+'/')
            elif stat.S_ISREG(info.st_mode):
                digest = hashlib.sha256()
                with open(entry.path, 'rb') as stream:
                    for block in iter(lambda: stream.read(1024*1024), b''): digest.update(block)
                row.update(kind='file', length=info.st_size, sha256=digest.hexdigest())
                if os.name == 'posix': row['mode'] = stat.S_IMODE(info.st_mode)
            else:
                raise ValueError('Unsupported fixture entry: ' + name)
            result.append(row)
    walk(root)
    return sorted(result, key=lambda r: r['path'])


def identity(rows: list[dict]) -> str:
    return hashlib.sha256(json.dumps(rows, sort_keys=True, ensure_ascii=True, separators=(',', ':')).encode()).hexdigest()
