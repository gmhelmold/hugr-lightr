"""Read-only C13 capacity observation; not recovery inventory or write authority.

Use an EXISTING directory. Missing paths fail instead of measuring an ancestor.
Native path semantics are preserved (including aliases and '..'); no lexical
normalization or Store initialization occurs. Counters describe the filesystem
of one retained directory, not a verified pool, quota or allocation-cost profile.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import errno
import os
import stat
import sys
import time
from typing import Callable

from resource_estimate import U64_MAX, InvalidInventory, capacities, mul

RAW_FIELDS = ("f_bsize", "f_frsize", "f_blocks", "f_bfree", "f_bavail",
              "f_files", "f_ffree", "f_favail", "f_flag", "f_namemax")
SENTINELS = {-1, (1 << 63) - 1, U64_MAX}


class Native:
    """Small injectable syscall boundary; no process-global test hooks."""
    def supported(self) -> bool:
        return (sys.platform.startswith("linux") or sys.platform == "darwin") and all(
            hasattr(os, name) for name in
            ("fstatvfs", "O_DIRECTORY", "O_CLOEXEC", "O_NONBLOCK"))

    def open(self, path: str | bytes) -> int:
        # No creation/writing. Resolve the supplied path natively, not abspath or
        # realpath followed by reopening. This is observation, NOT confinement.
        return os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NONBLOCK)

    def metadata(self, fd: int):
        return os.fstat(fd)

    def counters(self, fd: int):
        return os.fstatvfs(fd)

    def close(self, fd: int) -> None:
        os.close(fd)


def check() -> None:
    """Default cooperative checkpoint; callers may provide a bounded policy."""


def failure(phase: str, error: Exception) -> dict:
    result = {"phase": phase, "type": type(error).__name__, "message": str(error)}
    if isinstance(error, OSError) and error.errno is not None:
        result["errno"] = error.errno
    return result


def known(number: int) -> bool:
    return 0 <= number <= U64_MAX and number not in SENTINELS


def decode_counters(raw: dict) -> tuple[dict, list[str]]:
    """Translate only finite native counters; no quota/profile/capacity guesses."""
    if set(raw) != set(RAW_FIELDS) or any(
            type(n) is not int or not -1 <= n <= U64_MAX for n in raw.values()):
        raise InvalidInventory("invalid native statvfs counter")
    result = dict(free_bytes=None, quota_bytes=None, free_inodes=None)
    unknown = ["quota_not_inspected"]
    for total, free, available in (("f_blocks", "f_bfree", "f_bavail"),
                                   ("f_files", "f_ffree", "f_favail")):
        counts = [raw[name] for name in (total, free, available)]
        if all(known(n) for n in counts) and not counts[2] <= counts[1] <= counts[0]:
            raise InvalidInventory("inconsistent native counters: " + available)
    unit, available = raw["f_frsize"], raw["f_bavail"]
    if known(unit) and unit > 0 and known(available):
        # f_bfree includes blocks unavailable to ordinary users. f_bsize is NOT
        # a substitute for the accounting unit f_frsize; neither is a measured
        # physical allocation/metadata overhead profile.
        result["free_bytes"] = mul(unit, available)
    else:
        unknown.append("free_bytes_unavailable")
    if (known(raw["f_files"]) and raw["f_files"] > 0
            and known(raw["f_ffree"]) and known(raw["f_favail"])):
        result["free_inodes"] = raw["f_favail"]
    else:
        unknown.append("inode_accounting_unavailable")
    capacities(result)  # Preserve the existing estimator's axis/type contract.
    return result, unknown


def observe_directory(path: str | bytes | os.PathLike, *, native: Native | None = None,
                      checkpoint: Callable[[], None] = check) -> dict:
    """Observe through one owned fd; propagate failure/close uncertainty in data.

    Checkpoints cannot interrupt an already-running synchronous native call.
    The open fd retains an object, not a lock on its pathname or future capacity.
    Raw measurements may remain as diagnostics on error, but capacity projection
    is withheld. Close is attempted exactly once, even after an earlier error.
    """
    started = time.monotonic_ns()
    native = Native() if native is None else native
    record = {"schema": 1, "status": "OBSERVATION_FAILED", "exit_code": 2,
              "started_utc": datetime.now(timezone.utc).isoformat(),
              "platform": sys.platform, "python": sys.version.split()[0],
              "observation_accepted": False, "capacity": None,
              "errors": [], "descriptor_cleanup": "not_opened",
              "pool_identity_verified": False, "pathname_binding_verified": False,
              "writability_verified": False, "quota_inspected": False,
              "allocation_profile_measured": False, "inventory_complete": False,
              "retry_authorized": False, "runtime_qualified": False,
              "production_protocol_enabled": False, "scratch_reclamation_credit_bytes": 0}
    fd = None
    phase = "input"
    try:
        path = os.fspath(path)
        encoded = os.fsencode(path)
        if not encoded or b"\x00" in encoded or len(encoded) > 4096:
            raise ValueError("expected nonempty existing-directory path, at most 4096 bytes")
        record["requested_path"] = os.fsdecode(path)
        phase = "checkpoint_before_open"
        checkpoint()
        phase = "capability"
        if not native.supported():
            raise NotImplementedError("native capacity observation requires Linux/macOS statvfs")
        phase = "open"
        fd = native.open(path)
        record["descriptor_cleanup"] = "pending"
        phase = "checkpoint_after_open"
        checkpoint()
        phase = "identity"
        metadata = native.metadata(fd)
        if not stat.S_ISDIR(metadata.st_mode):
            raise NotADirectoryError(errno.ENOTDIR, "opened object is not a directory")
        record["directory_identity"] = {"device": metadata.st_dev, "inode": metadata.st_ino}
        phase = "checkpoint_before_measurement"
        checkpoint()
        phase = "statvfs"
        counters = native.counters(fd)
        raw = {name: getattr(counters, name) for name in RAW_FIELDS}
        phase = "decode"
        projection, unknown = decode_counters(raw)
        record["raw_statvfs"] = raw
        phase = "checkpoint_after_measurement"
        checkpoint()
        record["capacity"] = projection
        record["unknown_axes"] = unknown
    except (OSError, ValueError, TypeError, AttributeError, NotImplementedError) as error:
        record["errors"].append(failure(phase, error))
    finally:
        if fd is not None:
            try:
                native.close(fd)
                record["descriptor_cleanup"] = "closed"
            except OSError as error:
                # Failed close may or may not have released the fd. Retrying can
                # close an unrelated, reused descriptor; retain the uncertainty.
                record["descriptor_cleanup"] = "close_result_unknown"
                record["errors"].append(failure("close", error))
    if not record["errors"]:
        try:
            checkpoint()
        except (OSError, ValueError) as error:
            record["errors"].append(failure("checkpoint_after_close", error))
    if record["errors"]:
        record["capacity"] = None
        if record["errors"][0]["type"] == "NotImplementedError":
            record.update(status="UNSUPPORTED_CAPABILITY", exit_code=3)
    else:
        # A successful observation is NOT sufficient recovery information.
        # Quota remains unknown; the existing estimator will report INCOMPLETE.
        record.update(status="CAPACITY_OBSERVED", exit_code=0, observation_accepted=True)
    record["elapsed_ns"] = time.monotonic_ns() - started
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", required=True, help="existing directory to observe, read-only")
    args = parser.parse_args()
    record = observe_directory(args.directory)
    import json
    print(json.dumps(record, indent=2, sort_keys=True, ensure_ascii=True))
    return record["exit_code"]


if __name__ == "__main__":
    raise SystemExit(main())
