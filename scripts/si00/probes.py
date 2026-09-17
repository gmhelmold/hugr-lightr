"""Disposable OS capability probes; never claimed as Lightr protocol tests."""
from __future__ import annotations

import ctypes
import errno
import os
import platform
import shutil
import tempfile
from pathlib import Path


def capabilities(parent: Path) -> dict:
    result = {'scope': 'OS capability, not Lightr implementation proof', 'os': platform.system()}
    with tempfile.TemporaryDirectory(prefix='si00-probe-', dir=parent) as tmp:
        root = Path(tmp)
        src = root / 'src'; src.write_bytes(b'SI00-owned-fixture\n')
        with src.open('r+b') as stream:
            stream.flush(); os.fsync(stream.fileno())
        result['file_sync'] = 'EXERCISED'
        result['filesystem'] = filesystem(root)
        if os.name == 'posix':
            fd = os.open(root, os.O_RDONLY)
            try:
                os.fsync(fd); result['directory_sync'] = 'EXERCISED'
            finally:
                os.close(fd)
            src.chmod(0)
            try:
                try:
                    src.read_bytes(); result['permission_denial'] = 'NOT_EXERCISED_IDENTITY_BYPASSES'
                except PermissionError:
                    result['permission_denial'] = 'EXERCISED'
            finally:
                src.chmod(0o600)
        else:
            result['directory_sync'] = 'NOT_CLAIMED_WINDOWS'
            result['permission_denial'] = 'NOT_APPLICABLE_POSIX_MODE'
        directory = root / 'dir'; directory.mkdir()
        result['links'] = {}
        for name, target, is_dir in [('file', 'src', False), ('directory', 'dir', True),
                                     ('dangling_file', 'missing-file', False), ('dangling_directory', 'missing-dir', True)]:
            link = root / ('link-' + name)
            try:
                os.symlink(target, link, target_is_directory=is_dir)
                if os.readlink(link) != target: raise RuntimeError('link text changed')
                st = link.lstat()
                row = {'status': 'EXERCISED', 'target': target}
                if os.name == 'nt':
                    observed_dir = bool(st.st_file_attributes & 0x10)
                    if observed_dir != is_dir: raise RuntimeError('native link type mismatch')
                    row['native_directory_flag'] = observed_dir
                result['links'][name] = row
            except OSError as exc:
                result['links'][name] = {'status': 'NOT_EXERCISED', 'errno': exc.errno, 'winerror': getattr(exc, 'winerror', None)}
        cloned = root / 'cloned'
        try:
            if platform.system() == 'Linux':
                import fcntl
                with src.open('rb') as source, cloned.open('xb') as dest:
                    fcntl.ioctl(dest.fileno(), 0x40049409, source.fileno())
                method = 'FICLONE'
            elif platform.system() == 'Darwin':
                libc = ctypes.CDLL(None, use_errno=True)
                fn = libc.clonefile; fn.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_int]; fn.restype = ctypes.c_int
                if fn(os.fsencode(src), os.fsencode(cloned), 0) != 0:
                    err = ctypes.get_errno(); raise OSError(err, os.strerror(err))
                method = 'clonefile'
            else:
                raise OSError(errno.ENOTSUP, 'ReFS native clone not probed by this bootstrap')
            if cloned.read_bytes() != src.read_bytes(): raise RuntimeError('clone byte mismatch')
            result['native_clone'] = {'status': 'EXERCISED', 'method': method}
        except OSError as exc:
            result['native_clone'] = {'status': 'NOT_EXERCISED', 'errno': exc.errno}
        shutil.copyfile(src, root / 'copy')
        if (root / 'copy').read_bytes() != src.read_bytes(): raise RuntimeError('copy byte mismatch')
        result['copy_fallback'] = 'EXERCISED_SEPARATELY'
    return result


def filesystem(root: Path) -> dict:
    import subprocess
    if platform.system() == 'Linux':
        p = subprocess.run(['findmnt', '-n', '-o', 'FSTYPE', '-T', str(root)], capture_output=True, text=True)
        return {'type': p.stdout.strip() if p.returncode == 0 else 'UNKNOWN', 'device': root.stat().st_dev}
    if platform.system() == 'Darwin':
        import plistlib
        df = subprocess.run(['df', '-P', str(root)], capture_output=True, text=True, timeout=15)
        device = df.stdout.splitlines()[-1].split()[0] if df.returncode == 0 else str(root)
        p = subprocess.run(['diskutil', 'info', '-plist', device], capture_output=True, timeout=15)
        info = plistlib.loads(p.stdout) if p.returncode == 0 else {}
        return {'type': info.get('FilesystemType', info.get('FilesystemName', 'UNKNOWN')), 'device': root.stat().st_dev}
    if os.name == 'nt':
        kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        volume = ctypes.create_unicode_buffer(32768); fsname = ctypes.create_unicode_buffer(256)
        getpath = kernel.GetVolumePathNameW
        getpath.argtypes = [ctypes.c_wchar_p, ctypes.c_wchar_p, ctypes.c_uint32]
        getpath.restype = ctypes.c_int
        if not getpath(str(root), volume, len(volume)): return {'type': 'UNKNOWN'}
        info = kernel.GetVolumeInformationW
        info.argtypes = [ctypes.c_wchar_p, ctypes.c_wchar_p, ctypes.c_uint32, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_wchar_p, ctypes.c_uint32]
        info.restype = ctypes.c_int
        if not info(volume.value, None, 0, None, None, None, fsname, len(fsname)): return {'type': 'UNKNOWN'}
        return {'type': fsname.value, 'device': root.stat().st_dev}
    return {'type': 'UNKNOWN'}
