"""C13 read-only capacity arithmetic over a caller-supplied recovery inventory.

This does NOT inspect a Store, verify an evidence hash, acquire a lease, reclaim
scratch, discover dependencies, authorize a retry, or certify recovery. All
physical allocation costs and capacity pools must come from the caller's native
inspection/measurement. No universal free-space constant or CoW credit is used.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from typing import Any, BinaryIO

U64_MAX = (1 << 64) - 1
MAX_INPUT_BYTES = 8 * 1024 * 1024
MAX_POOLS = 128
MAX_DOMAINS = 1024
MAX_JSON_DEPTH = 16
MAX_ITEMS = 100_000
MAX_METADATA_BYTES = 1_048_576 + 44  # ADR-0020 largest body plus framing.
AXES = ("free_bytes", "quota_bytes", "free_inodes")
PROFILE_FIELDS = ("allocation_unit_bytes", "file_overhead_bytes",
                  "directory_bytes", "inodes_per_file", "inodes_per_directory")


class InvalidInventory(ValueError):
    """An invalid or internally inconsistent inventory cannot yield an estimate."""


def require(ok: bool, message: str) -> None:
    if not ok:
        raise InvalidInventory(message)


def fields(value: Any, expected: set[str], where: str) -> dict:
    require(type(value) is dict, where + ": expected object")
    require(set(value) == expected, where + ": missing or unknown fields")
    return value


def uint(value: Any, where: str) -> int:
    require(type(value) is int and 0 <= value <= U64_MAX, where + ": expected u64")
    return value


def add(left: int, right: int) -> int:
    return uint(left + right, "sum overflow")


def mul(left: int, right: int) -> int:
    return uint(left * right, "product overflow")


def identifier(value: Any, where: str) -> str:
    require(type(value) is str and re.fullmatch(r"[A-Za-z0-9_.:-]{1,128}", value)
            is not None, where + ": invalid identity label")
    return value


def digest(value: Any, length: int, where: str) -> str:
    require(type(value) is str and re.fullmatch(r"[0-9a-f]{%d}" % length, value)
            is not None, where + ": invalid lowercase digest")
    return value


def array(value: Any, maximum: int, where: str) -> list:
    require(type(value) is list and len(value) <= maximum, where + ": invalid array")
    return value


def rounded(size: int, unit: int) -> int:
    blocks, remainder = divmod(size, unit)
    return mul(blocks + bool(remainder), unit)


def pairs(items: list[tuple[str, Any]]) -> dict:
    result: dict = {}
    for key, value in items:
        require(key not in result, "duplicate JSON key: " + key)
        result[key] = value
    return result


def load(stream: BinaryIO) -> dict:
    data = stream.read(MAX_INPUT_BYTES + 1)
    require(len(data) <= MAX_INPUT_BYTES, "input exceeds byte limit")
    # Bound structure before decoding, independently of interpreter recursion
    # limits. Brackets in JSON strings (including escaped quotes) are data.
    depth = 0
    quoted = escaped = False
    for char in data:
        if quoted:
            if escaped:
                escaped = False
            elif char == 92:
                escaped = True
            elif char == 34:
                quoted = False
        elif char == 34:
            quoted = True
        elif char in (91, 123):
            depth += 1
            require(depth <= MAX_JSON_DEPTH, "JSON nesting limit exceeded")
        elif char in (93, 125):
            depth -= 1
    # parse_int bounds conversion before Python's own digit-limit handling.
    def integer(text: str) -> int:
        require(len(text) <= 21, "integer token exceeds u64 width")
        return int(text)
    def forbidden(token: str) -> Any:
        raise InvalidInventory("non-integer JSON number: " + token[:40])
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=pairs,
                          parse_int=integer, parse_float=forbidden,
                          parse_constant=forbidden)
    except (UnicodeError, json.JSONDecodeError, RecursionError) as error:
        raise InvalidInventory("invalid or excessively nested UTF-8 JSON") from error


def profile(value: Any) -> dict | None:
    if value is None:
        return None
    fields(value, set(PROFILE_FIELDS) | {"measurement_sha256"}, "allocation profile")
    digest(value["measurement_sha256"], 64, "measurement reference")
    for name in PROFILE_FIELDS:
        uint(value[name], "profile." + name)
    unit = value["allocation_unit_bytes"]
    require(unit > 0 and unit & (unit - 1) == 0, "allocation unit must be a power of two")
    return value


def capacities(value: Any) -> dict:
    fields(value, set(AXES), "capacity")
    for axis in AXES:
        item = value[axis]
        if item is None:
            continue
        if type(item) is str:
            require(axis != "free_bytes" and item == "not_applicable",
                    "only quota/inodes can be explicitly not_applicable")
        else:
            uint(item, "capacity." + axis)
    return value


def domain(value: Any, pools: dict) -> dict:
    fields(value, {"id", "kind", "pool_id", "inventory_complete", "objects",
                   "metadata", "directories"}, "domain")
    name = identifier(value["id"], "domain.id")
    require(value["kind"] in ("store", "cache", "workspace"), "unknown resource domain")
    pool_id = identifier(value["pool_id"], "domain.pool_id")
    require(pool_id in pools, "unknown capacity pool")
    require(type(value["inventory_complete"]) is bool, "inventory_complete is not bool")
    objects = array(value["objects"], MAX_ITEMS, "objects")
    metadata = array(value["metadata"], MAX_ITEMS, "metadata")
    directories = uint(value["directories"], "directories")
    seen: dict[str, tuple[int, str]] = {}
    sizes: list[tuple[int, int]] = []
    rewrites = logical_payload = unknown = 0
    for obj in objects:
        fields(obj, {"digest", "length", "state"}, "object")
        key = digest(obj["digest"], 64, "object digest")
        length = uint(obj["length"], "object length")
        state = obj["state"]
        require(state in ("rewrite", "confirmed", "unknown"), "unknown object state")
        identity = (length, state)
        if key in seen:
            require(seen[key] == identity, "conflicting duplicate digest within a domain")
            continue
        seen[key] = identity
        if state == "rewrite":
            rewrites = add(rewrites, 1)
            logical_payload = add(logical_payload, length)
            sizes.append((length, 1))
        elif state == "unknown":
            unknown += 1
    names: set[str] = set()
    logical_metadata = metadata_files = 0
    for record in metadata:
        fields(record, {"id", "bytes", "count"}, "metadata allocation")
        key = identifier(record["id"], "metadata.id")
        require(key not in names, "duplicate metadata allocation identity")
        names.add(key)
        size = uint(record["bytes"], "metadata bytes")
        count = uint(record["count"], "metadata count")
        require(size <= MAX_METADATA_BYTES and count > 0, "invalid metadata bound/count")
        logical_metadata = add(logical_metadata, mul(size, count))
        metadata_files = add(metadata_files, count)
        sizes.append((size, count))
    logical = add(logical_payload, logical_metadata)
    files = add(rewrites, metadata_files)
    p = pools[pool_id]["profile"]
    estimated_bytes = estimated_inodes = None
    if p is not None:
        physical = 0
        for size, count in sizes:
            allocation = add(rounded(size, p["allocation_unit_bytes"]),
                             p["file_overhead_bytes"])
            physical = add(physical, mul(allocation, count))
        estimated_bytes = add(physical, mul(directories, p["directory_bytes"]))
        estimated_inodes = add(mul(files, p["inodes_per_file"]),
                               mul(directories, p["inodes_per_directory"]))
    complete = value["inventory_complete"] and unknown == 0 and p is not None
    return {"id": name, "kind": value["kind"], "pool_id": pool_id,
            "caller_inventory_complete": value["inventory_complete"],
            "estimate_complete": complete, "unknown_objects": unknown,
            "rewrite_objects": rewrites, "metadata_files": metadata_files,
            "new_directories": directories, "logical_rewrite_bytes": logical_payload,
            "logical_metadata_bytes": logical_metadata, "known_logical_bytes": logical,
            "known_allocation_bytes": estimated_bytes,
            "known_allocation_inodes": estimated_inodes,
            "additional_bytes": estimated_bytes if complete else None,
            "additional_inodes": estimated_inodes if complete else None}


def estimate(value: Any) -> dict:
    fields(value, {"schema", "inspection_sha256", "pools", "domains"}, "inventory")
    require(type(value["schema"]) is int and value["schema"] == 1, "unsupported schema")
    digest(value["inspection_sha256"], 64, "inspection reference")
    pools: dict[str, dict] = {}
    for item in array(value["pools"], MAX_POOLS, "pools"):
        fields(item, {"id", "profile", "capacity"}, "pool")
        name = identifier(item["id"], "pool.id")
        require(name not in pools, "duplicate pool identity")
        pools[name] = {"profile": profile(item["profile"]),
                       "capacity": capacities(item["capacity"])}
    rows = array(value["domains"], MAX_DOMAINS, "domains")
    require(bool(rows) and bool(pools), "empty inventory is not complete recovery evidence")
    # Apply a global item budget, not merely one budget per domain.
    count = 0
    for item in rows:
        require(type(item) is dict, "domain must be object")
        count += len(array(item.get("objects"), MAX_ITEMS, "objects"))
        count += len(array(item.get("metadata"), MAX_ITEMS, "metadata"))
        require(count <= MAX_ITEMS, "global item budget exceeded")
    domains = [domain(item, pools) for item in rows]
    require(len({item["id"] for item in domains}) == len(domains), "duplicate domain identity")
    require({item["pool_id"] for item in domains} == set(pools), "unused capacity pool")
    grouped = []
    for name in sorted(pools):
        p = pools[name]
        members = [item for item in domains if item["pool_id"] == name]
        logical = known_bytes = known_inodes = 0
        for item in members:
            logical = add(logical, item["known_logical_bytes"])
            known_bytes = add(known_bytes, item["known_allocation_bytes"] or 0)
            known_inodes = add(known_inodes, item["known_allocation_inodes"] or 0)
        complete = all(item["estimate_complete"] for item in members)
        comparisons = {}
        for axis in AXES:
            available = p["capacity"][axis]
            need = known_inodes if axis == "free_inodes" else known_bytes
            if available == "not_applicable":
                status = "CALLER_DECLARED_NOT_APPLICABLE"
            elif available is None or p["profile"] is None:
                status = "UNKNOWN"
            elif available < need:
                status = "BELOW_KNOWN_ESTIMATE"
            elif not complete:
                status = "UNKNOWN"
            else:
                status = "AT_LEAST_ESTIMATE"
            comparisons[axis] = {"observed": available, "comparison": status,
                                 "shortfall": need - available
                                 if status == "BELOW_KNOWN_ESTIMATE" else None}
        grouped.append({"id": name, "domain_ids": sorted(item["id"] for item in members),
                        "estimate_complete": complete, "known_logical_bytes": logical,
                        "known_allocation_bytes": known_bytes if p["profile"] else None,
                        "known_allocation_inodes": known_inodes if p["profile"] else None,
                        "additional_bytes": known_bytes if complete else None,
                        "additional_inodes": known_inodes if complete else None,
                        "measurement_sha256": p["profile"]["measurement_sha256"]
                        if p["profile"] else None, "capacity": comparisons})
    statuses = [axis["comparison"] for item in grouped for axis in item["capacity"].values()]
    if "BELOW_KNOWN_ESTIMATE" in statuses:
        status, exit_code = "OBSERVED_RESOURCE_SHORTFALL", 4
    elif not all(item["estimate_complete"] for item in grouped) or "UNKNOWN" in statuses:
        status, exit_code = "INCOMPLETE_INFORMATION", 3
    else:
        status, exit_code = "ESTIMATE_ONLY", 0
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()
    return {"schema": 1, "status": status, "exit_code": exit_code,
            "inventory_sha256": hashlib.sha256(encoded).hexdigest(),
            "inspection_sha256": value["inspection_sha256"],
            "evidence_references_verified": False, "retry_authorized": False,
            "runtime_qualified": False, "production_protocol_enabled": False,
            "scratch_reclamation_credit_bytes": 0,
            "domains": sorted(domains, key=lambda row: row["id"]), "pools": grouped}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", default="-", help="inventory JSON file; '-' reads stdin")
    args = parser.parse_args()
    try:
        if args.input == "-":
            value = load(sys.stdin.buffer)
        else:
            with open(args.input, "rb") as stream:
                value = load(stream)
        result = estimate(value)
    except (InvalidInventory, OSError, MemoryError, RecursionError) as error:
        print(json.dumps({"schema": 1, "status": "INVALID_INVENTORY", "exit_code": 2,
                          "retry_authorized": False, "error": str(error)}))
        return 2
    print(json.dumps(result, indent=2, sort_keys=True))
    return result["exit_code"]


if __name__ == "__main__":
    raise SystemExit(main())
