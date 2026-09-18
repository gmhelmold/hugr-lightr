"""Executable C13 arithmetic controls, NOT a native-resource or recovery witness."""
from __future__ import annotations

import copy
import importlib.util
import io
import json
import random
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/si01/resource_estimate.py"
SPEC = importlib.util.spec_from_file_location("resource_estimate", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
resource = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(resource)


def fixture() -> dict:
    return {"schema": 1, "inspection_sha256": "a" * 64,
            "pools": [{"id": "fs:1", "profile": {
                "measurement_sha256": "b" * 64, "allocation_unit_bytes": 4096,
                "file_overhead_bytes": 128, "directory_bytes": 4096,
                "inodes_per_file": 1, "inodes_per_directory": 1},
                "capacity": {"free_bytes": 1_000_000, "quota_bytes": "not_applicable",
                             "free_inodes": 100}}],
            "domains": [{"id": "store:a", "kind": "store", "pool_id": "fs:1",
                         "inventory_complete": True,
                         "objects": [{"digest": "1" * 64, "length": 4097, "state": "rewrite"},
                                     {"digest": "2" * 64, "length": 8193, "state": "rewrite"}],
                         "metadata": [{"id": "receipt-1", "bytes": 80, "count": 2}],
                         "directories": 3}]}


class ResourceEstimateTests(unittest.TestCase):
    def setUp(self):
        self.value = fixture()

    def run_estimate(self):
        return resource.estimate(self.value)

    def test_all_rewrites_and_metadata_are_summed_not_largest_file(self):
        result = self.run_estimate()
        row, pool = result["domains"][0], result["pools"][0]
        self.assertEqual(row["logical_rewrite_bytes"], 4097 + 8193)
        self.assertEqual(row["logical_metadata_bytes"], 2 * 80)
        expected = (8192 + 128) + (12288 + 128) + 2 * (4096 + 128) + 3 * 4096
        self.assertEqual(row["additional_bytes"], expected)
        self.assertEqual(pool["additional_bytes"], expected)
        self.assertEqual(pool["additional_inodes"], 7)
        self.assertEqual(result["status"], "ESTIMATE_ONLY")
        for field in ("retry_authorized", "evidence_references_verified",
                      "runtime_qualified", "production_protocol_enabled"):
            self.assertIs(result[field], False)
        self.assertEqual(result["scratch_reclamation_credit_bytes"], 0)

    def test_identical_dependencies_are_deduplicated_within_one_domain(self):
        before = self.run_estimate()
        self.value["domains"][0]["objects"] *= 3
        after = self.run_estimate()
        self.assertEqual(after["domains"], before["domains"])
        self.assertEqual(after["pools"], before["pools"])
        self.assertNotEqual(after["inventory_sha256"], before["inventory_sha256"])

    def test_conflicting_duplicate_length_or_state_is_rejected(self):
        original = self.value["domains"][0]["objects"][0]
        for field, value in (("length", 1), ("state", "confirmed"), ("state", "unknown")):
            with self.subTest(field=field, value=value):
                self.setUp()
                other = dict(original, **{field: value})
                self.value["domains"][0]["objects"].append(other)
                with self.assertRaisesRegex(resource.InvalidInventory, "conflicting"):
                    self.run_estimate()

    def test_separate_stores_sharing_one_disk_sum_before_capacity_check(self):
        single_cost = self.run_estimate()["pools"][0]["additional_bytes"]
        second = copy.deepcopy(self.value["domains"][0])
        second["id"] = "store:b"
        self.value["domains"].append(second)
        self.value["pools"][0]["capacity"]["free_bytes"] = single_cost + 1
        result = self.run_estimate()
        self.assertEqual(result["pools"][0]["additional_bytes"], 2 * single_cost)
        self.assertEqual(result["status"], "OBSERVED_RESOURCE_SHORTFALL")
        self.assertEqual(result["exit_code"], 4)
        self.assertEqual(result["pools"][0]["capacity"]["free_bytes"]["shortfall"], single_cost - 1)

    def test_separate_pools_cannot_pay_each_others_shortfall(self):
        second_pool = copy.deepcopy(self.value["pools"][0])
        second_pool["id"] = "fs:2"
        self.value["pools"].append(second_pool)
        second_domain = copy.deepcopy(self.value["domains"][0])
        second_domain.update(id="cache:b", kind="cache", pool_id="fs:2")
        self.value["domains"].append(second_domain)
        self.value["pools"][0]["capacity"]["free_bytes"] = 1
        result = self.run_estimate()
        self.assertEqual(len(result["pools"]), 2)
        self.assertEqual(result["status"], "OBSERVED_RESOURCE_SHORTFALL")
        self.assertEqual(result["pools"][1]["capacity"]["free_bytes"]["comparison"],
                         "AT_LEAST_ESTIMATE")

    def test_quota_and_inode_limits_are_independent_from_free_bytes(self):
        for axis in ("quota_bytes", "free_inodes"):
            with self.subTest(axis=axis):
                self.setUp()
                self.value["pools"][0]["capacity"][axis] = 0
                result = self.run_estimate()
                self.assertEqual(result["status"], "OBSERVED_RESOURCE_SHORTFALL")
                self.assertEqual(result["pools"][0]["capacity"]["free_bytes"]["comparison"],
                                 "AT_LEAST_ESTIMATE")
                self.assertEqual(result["pools"][0]["capacity"][axis]["comparison"],
                                 "BELOW_KNOWN_ESTIMATE")

    def test_exact_observed_capacity_equals_estimate_not_shortfall(self):
        cost = self.run_estimate()["pools"][0]
        self.value["pools"][0]["capacity"] = {
            "free_bytes": cost["additional_bytes"], "quota_bytes": cost["additional_bytes"],
            "free_inodes": cost["additional_inodes"]}
        self.assertEqual(self.run_estimate()["status"], "ESTIMATE_ONLY")

    def test_unknown_state_never_becomes_confirmed_or_free(self):
        self.value["domains"][0]["objects"][0]["state"] = "unknown"
        result = self.run_estimate()
        self.assertEqual(result["status"], "INCOMPLETE_INFORMATION")
        self.assertIsNone(result["pools"][0]["additional_bytes"])
        self.assertEqual(result["domains"][0]["unknown_objects"], 1)
        self.assertGreater(result["pools"][0]["known_allocation_bytes"], 0)

    def test_incomplete_inventory_preserves_known_cost_without_acceptance(self):
        cost = self.run_estimate()["pools"][0]["additional_bytes"]
        self.value["domains"][0]["inventory_complete"] = False
        result = self.run_estimate()
        self.assertEqual(result["status"], "INCOMPLETE_INFORMATION")
        self.assertEqual(result["pools"][0]["known_allocation_bytes"], cost)
        self.assertIsNone(result["pools"][0]["additional_bytes"])

    def test_missing_profile_is_not_zero_overhead(self):
        self.value["pools"][0]["profile"] = None
        result = self.run_estimate()
        self.assertEqual(result["status"], "INCOMPLETE_INFORMATION")
        self.assertIsNone(result["pools"][0]["additional_bytes"])
        self.assertIsNone(result["pools"][0]["known_allocation_bytes"])
        self.assertGreater(result["pools"][0]["known_logical_bytes"], 0)

    def test_unknown_capacity_axes_are_not_infinite(self):
        for axis in resource.AXES:
            with self.subTest(axis=axis):
                self.setUp()
                self.value["pools"][0]["capacity"][axis] = None
                result = self.run_estimate()
                self.assertEqual(result["status"], "INCOMPLETE_INFORMATION")
                self.assertEqual(result["pools"][0]["capacity"][axis]["comparison"], "UNKNOWN")

    def test_known_shortfall_is_reported_even_when_inventory_is_incomplete(self):
        self.value["domains"][0]["inventory_complete"] = False
        self.value["pools"][0]["capacity"]["free_bytes"] = 1
        self.assertEqual(self.run_estimate()["status"], "OBSERVED_RESOURCE_SHORTFALL")

    def test_confirmed_objects_do_not_erase_planned_receipt_costs(self):
        for obj in self.value["domains"][0]["objects"]:
            obj["state"] = "confirmed"
        result = self.run_estimate()
        self.assertEqual(result["domains"][0]["rewrite_objects"], 0)
        self.assertEqual(result["domains"][0]["logical_rewrite_bytes"], 0)
        self.assertEqual(result["domains"][0]["metadata_files"], 2)
        self.assertEqual(result["pools"][0]["additional_bytes"], 2 * (4096 + 128) + 3 * 4096)

    def test_empty_payload_still_counts_owned_file_and_inode(self):
        self.value["domains"][0].update(objects=[{"digest": "3" * 64, "length": 0,
                                                 "state": "rewrite"}], metadata=[], directories=0)
        result = self.run_estimate()["pools"][0]
        self.assertEqual(result["additional_bytes"], 128)
        self.assertEqual(result["additional_inodes"], 1)

    def test_no_allocations_does_not_authorize_zero_space_recovery(self):
        self.value["domains"][0].update(objects=[], metadata=[], directories=0)
        self.value["pools"][0]["capacity"] = {"free_bytes": 0, "quota_bytes": 0, "free_inodes": 0}
        result = self.run_estimate()
        self.assertEqual(result["status"], "ESTIMATE_ONLY")
        self.assertEqual(result["pools"][0]["additional_bytes"], 0)
        self.assertFalse(result["retry_authorized"])

    def test_bool_negative_float_and_oversize_are_not_u64(self):
        for bad in (True, False, -1, 1.0, "1", None, resource.U64_MAX + 1):
            with self.subTest(value=bad):
                self.setUp()
                self.value["domains"][0]["objects"][0]["length"] = bad
                with self.assertRaises(resource.InvalidInventory):
                    self.run_estimate()

    def test_overflow_is_explicit_in_sum_product_and_rounding(self):
        cases = (
            lambda v: v["domains"][0]["objects"][0].update(length=resource.U64_MAX),
            lambda v: v["domains"][0]["metadata"][0].update(count=resource.U64_MAX),
            lambda v: v["domains"][0].update(directories=resource.U64_MAX),
            lambda v: v["pools"][0]["profile"].update(file_overhead_bytes=resource.U64_MAX),
            lambda v: v["pools"][0]["profile"].update(inodes_per_file=resource.U64_MAX),
        )
        for mutate in cases:
            self.setUp()
            mutate(self.value)
            with self.assertRaisesRegex(resource.InvalidInventory, "overflow"):
                self.run_estimate()

    def test_pool_sum_overflow_is_rejected_even_if_each_domain_fits(self):
        p = self.value["pools"][0]["profile"]
        p.update(allocation_unit_bytes=1, file_overhead_bytes=0)
        d = self.value["domains"][0]
        d.update(objects=[{"digest": "1" * 64, "length": 1 << 63, "state": "rewrite"}],
                 metadata=[], directories=0)
        other = copy.deepcopy(d)
        other["id"] = "store:b"
        self.value["domains"].append(other)
        with self.assertRaisesRegex(resource.InvalidInventory, "overflow"):
            self.run_estimate()

    def test_alignment_rounds_each_allocation_not_only_the_total(self):
        d = self.value["domains"][0]
        d["metadata"] = []
        d["directories"] = 0
        for obj in d["objects"]:
            obj["length"] = 1
        p = self.value["pools"][0]["profile"]
        p["file_overhead_bytes"] = 0
        self.assertEqual(self.run_estimate()["pools"][0]["additional_bytes"], 8192)

    def test_invalid_allocation_unit_or_measurement_is_rejected(self):
        for unit in (0, 3, True, -1, "4096"):
            self.setUp()
            self.value["pools"][0]["profile"]["allocation_unit_bytes"] = unit
            with self.assertRaises(resource.InvalidInventory):
                self.run_estimate()
        self.setUp()
        self.value["pools"][0]["profile"]["measurement_sha256"] = ""
        with self.assertRaises(resource.InvalidInventory):
            self.run_estimate()

    def test_metadata_limit_zero_count_and_duplicate_are_rejected(self):
        for edit in ({"bytes": resource.MAX_METADATA_BYTES + 1}, {"count": 0}):
            self.setUp()
            self.value["domains"][0]["metadata"][0].update(edit)
            with self.assertRaises(resource.InvalidInventory):
                self.run_estimate()
        self.setUp()
        self.value["domains"][0]["metadata"] *= 2
        with self.assertRaisesRegex(resource.InvalidInventory, "duplicate metadata"):
            self.run_estimate()

    def test_unknown_fields_never_allow_reclamation_or_clone_discounts(self):
        for key in ("scratch_bytes", "cow_ratio", "reclaimable", "emergency_reserve"):
            self.setUp()
            self.value["domains"][0][key] = 1_000_000
            with self.assertRaisesRegex(resource.InvalidInventory, "unknown fields"):
                self.run_estimate()

    def test_empty_duplicate_unknown_domain_and_unused_pool_are_rejected(self):
        cases = (
            lambda v: v.update(domains=[]),
            lambda v: v.update(pools=[]),
            lambda v: v["domains"].append(copy.deepcopy(v["domains"][0])),
            lambda v: v["pools"].append(copy.deepcopy(v["pools"][0])),
            lambda v: v["domains"][0].update(pool_id="missing"),
            lambda v: v["domains"][0].update(kind="foreign-lease"),
            lambda v: v["pools"].append(dict(v["pools"][0], id="unused")),
            lambda v: v["domains"][0].update(id="/tmp/not-an-identity"),
        )
        for mutate in cases:
            self.setUp()
            mutate(self.value)
            with self.assertRaises(resource.InvalidInventory):
                self.run_estimate()

    def test_bad_digests_schema_state_and_completion_flag_are_rejected(self):
        cases = (
            lambda v: v.update(schema=True),
            lambda v: v.update(schema=2),
            lambda v: v.update(inspection_sha256="A" * 64),
            lambda v: v["domains"][0]["objects"][0].update(digest="0" * 63),
            lambda v: v["domains"][0]["objects"][0].update(state="exists"),
            lambda v: v["domains"][0].update(inventory_complete=1),
            lambda v: v["pools"][0]["capacity"].update(free_bytes="not_applicable"),
            lambda v: v["pools"][0]["capacity"].update(quota_bytes="unlimited"),
        )
        for mutate in cases:
            self.setUp()
            mutate(self.value)
            with self.assertRaises(resource.InvalidInventory):
                self.run_estimate()

    def test_global_item_limit_is_not_multiplied_by_domains(self):
        old = resource.MAX_ITEMS
        try:
            resource.MAX_ITEMS = 5
            other = copy.deepcopy(self.value["domains"][0])
            other["id"] = "store:b"
            self.value["domains"].append(other)
            with self.assertRaisesRegex(resource.InvalidInventory, "global item"):
                self.run_estimate()
        finally:
            resource.MAX_ITEMS = old

    def test_input_is_unchanged_and_key_order_does_not_change_hash(self):
        before = copy.deepcopy(self.value)
        a = self.run_estimate()
        self.assertEqual(before, self.value)
        reordered = dict(reversed(list(self.value.items())))
        self.assertEqual(a, resource.estimate(reordered))

    def test_json_rejects_duplicates_nonintegers_utf8_and_trailing_garbage(self):
        for data in (b'{"x":1,"x":2}', b'{"x":{"y":0,"y":1}}', b'NaN', b'Infinity',
                     b'1.0', b'1e3', b'\xff', b'{} trailing', b'{', b'9' * 22,
                     b'[' * 1200 + b']' * 1200):
            with self.subTest(data=data[:40]):
                with self.assertRaises(resource.InvalidInventory):
                    resource.load(io.BytesIO(data))

    def test_nesting_limit_is_explicit_and_ignores_escaped_string_brackets(self):
        legal = {"text": 'brackets [{{ and escaped quote " plus slash \\ ' * 100}
        self.assertEqual(resource.load(io.BytesIO(json.dumps(legal).encode())), legal)
        inside = b'[' * resource.MAX_JSON_DEPTH + b'0' + b']' * resource.MAX_JSON_DEPTH
        self.assertIsInstance(resource.load(io.BytesIO(inside)), list)
        with self.assertRaisesRegex(resource.InvalidInventory, "nesting limit"):
            resource.load(io.BytesIO(b'[' + inside + b']'))

    def test_byte_limit_reads_only_one_bounded_chunk(self):
        class Guarded(io.BytesIO):
            def read(self, size=-1):
                self.request = size
                return super().read(size)
        stream = Guarded(b' ' * (resource.MAX_INPUT_BYTES + 2))
        with self.assertRaisesRegex(resource.InvalidInventory, "byte limit"):
            resource.load(stream)
        self.assertEqual(stream.request, resource.MAX_INPUT_BYTES + 1)
        self.assertEqual(stream.tell(), resource.MAX_INPUT_BYTES + 1)

    def test_seeded_arithmetic_against_an_independent_small_input_oracle(self):
        rng = random.Random(160013)
        for sample in range(400):
            with self.subTest(sample=sample):
                value = fixture()
                p = value["pools"][0]["profile"]
                p.update(allocation_unit_bytes=1 << rng.randrange(0, 13),
                         file_overhead_bytes=rng.randrange(257),
                         directory_bytes=rng.randrange(8193))
                value["domains"] = []
                expected_bytes = expected_inodes = 0
                for n in range(rng.randrange(1, 5)):
                    sizes = [rng.randrange(50000) for _ in range(rng.randrange(7))]
                    metadata = [(rng.randrange(1000), rng.randrange(1, 5))
                                for _ in range(rng.randrange(4))]
                    dirs = rng.randrange(5)
                    value["domains"].append({
                        "id": f"store:{n}", "kind": "store", "pool_id": "fs:1",
                        "inventory_complete": True,
                        "objects": [{"digest": format(i, "064x"), "length": size,
                                     "state": "rewrite"} for i, size in enumerate(sizes)],
                        "metadata": [{"id": f"meta:{i}", "bytes": size, "count": count}
                                     for i, (size, count) in enumerate(metadata)],
                        "directories": dirs})
                    # Independent formula; no production summation/rounding helpers.
                    unit = p["allocation_unit_bytes"]
                    each_file = sizes + [size for size, count in metadata for _ in range(count)]
                    expected_bytes += sum(((size + unit - 1) // unit) * unit +
                                          p["file_overhead_bytes"] for size in each_file)
                    expected_bytes += dirs * p["directory_bytes"]
                    expected_inodes += len(each_file) + dirs
                pool = resource.estimate(value)["pools"][0]
                self.assertEqual(pool["additional_bytes"], expected_bytes)
                self.assertEqual(pool["additional_inodes"], expected_inodes)

    def test_metadata_exact_bound_and_not_applicable_axes_remain_explicit(self):
        self.value["domains"][0]["metadata"][0]["bytes"] = resource.MAX_METADATA_BYTES
        self.value["pools"][0]["capacity"].update(free_bytes=100_000_000,
                                                  free_inodes="not_applicable")
        result = self.run_estimate()
        self.assertEqual(result["status"], "ESTIMATE_ONLY")
        for axis in ("quota_bytes", "free_inodes"):
            self.assertEqual(result["pools"][0]["capacity"][axis]["comparison"],
                             "CALLER_DECLARED_NOT_APPLICABLE")
        self.assertFalse(result["evidence_references_verified"])

    def test_cli_exit_codes_stdout_and_no_directory_mutation(self):
        with tempfile.TemporaryDirectory() as tmp:
            parent = Path(tmp)
            sentinel = parent / "sentinel"
            sentinel.write_bytes(b"preserve me")
            for mode, expected in (("complete", 0), ("unknown", 3), ("shortfall", 4), ("bad", 2)):
                value = fixture()
                if mode == "unknown":
                    value["pools"][0]["profile"] = None
                elif mode == "shortfall":
                    value["pools"][0]["capacity"]["free_bytes"] = 0
                elif mode == "bad":
                    value["schema"] = 2
                source = parent / "inventory.json"
                source.write_text(json.dumps(value), encoding="utf-8")
                before = {p.name: p.read_bytes() for p in parent.iterdir()}
                result = subprocess.run([sys.executable, str(SCRIPT), "--input", str(source)],
                                        cwd=parent, capture_output=True, timeout=10)
                self.assertEqual(result.returncode, expected, result.stderr.decode())
                output = json.loads(result.stdout)
                self.assertEqual(output["exit_code"], expected)
                self.assertFalse(output["retry_authorized"])
                self.assertEqual(result.stderr, b"")
                self.assertEqual(before, {p.name: p.read_bytes() for p in parent.iterdir()})

    def test_cli_stdin_and_missing_file_fail_explicitly(self):
        result = subprocess.run([sys.executable, str(SCRIPT)], input=json.dumps(fixture()).encode(),
                                capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["status"], "ESTIMATE_ONLY")
        with tempfile.TemporaryDirectory() as tmp:
            missing = Path(tmp) / "not-present.json"
            result = subprocess.run([sys.executable, str(SCRIPT), "--input", str(missing)],
                                    capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(json.loads(result.stdout)["status"], "INVALID_INVENTORY")
            self.assertFalse(missing.exists())


class FragmentedReader(io.BytesIO):
    """Blocking binary stream with legal short reads; never a real Store."""
    def __init__(self, content, chunk):
        super().__init__(content)
        self.chunk = chunk
        self.requests = []

    def read(self, size=-1):
        if size <= 0:
            raise AssertionError("every read must have a positive finite budget")
        self.requests.append(size)
        return super().read(min(size, self.chunk))


class ResourceStreamTests(unittest.TestCase):
    def test_fragmented_stream_matches_complete_inventory(self):
        value = fixture()
        encoded = json.dumps(value).encode("utf-8")
        for chunk in (1, 7, 64, len(encoded)):
            with self.subTest(chunk=chunk):
                stream = FragmentedReader(encoded, chunk)
                actual = resource.load(stream)
                self.assertEqual(actual, value)
                self.assertEqual(resource.estimate(actual), resource.estimate(value))
                self.assertEqual(stream.tell(), len(encoded))
                self.assertGreater(len(stream.requests), 1)

    def test_valid_prefix_cannot_hide_trailing_fragment(self):
        encoded = json.dumps(fixture()).encode("utf-8")
        for suffix in (b" trailing", b" {}", b"\xff"):
            with self.subTest(suffix=suffix):
                stream = FragmentedReader(encoded + suffix, len(encoded))
                with self.assertRaises(resource.InvalidInventory):
                    resource.load(stream)
                self.assertEqual(stream.tell(), len(encoded) + len(suffix))

    def test_late_io_error_is_not_hidden_by_valid_prefix(self):
        encoded = json.dumps(fixture()).encode("utf-8")
        failure = OSError(5, "controlled late read failure")

        class LateFailure(io.BytesIO):
            def read(self, size=-1):
                if self.tell() == len(encoded):
                    raise failure
                return super().read(size)

        with self.assertRaises(OSError) as raised:
            resource.load(LateFailure(encoded))
        self.assertIs(raised.exception, failure)

    def test_fragmented_total_limit_stops_after_one_extra_byte(self):
        from unittest.mock import patch
        stream = FragmentedReader(b"{}" + b" " * 30, 2)
        with patch.object(resource, "MAX_INPUT_BYTES", 9):
            with self.assertRaisesRegex(resource.InvalidInventory, "byte limit"):
                resource.load(stream)
        self.assertEqual(stream.tell(), 10)
        self.assertEqual(stream.requests, [10, 8, 6, 4, 2])

    def test_exact_byte_limit_checks_eof_without_exceeding_budget(self):
        from unittest.mock import patch
        stream = FragmentedReader(b"{}", 1)
        with patch.object(resource, "MAX_INPUT_BYTES", 2):
            self.assertEqual(resource.load(stream), {})
        self.assertEqual(stream.requests, [3, 2, 1])
        self.assertEqual(stream.tell(), 2)

    def test_nonblocking_or_invalid_read_results_fail_explicitly(self):
        class InvalidReader:
            def __init__(self, result):
                self.result = result
            def read(self, size=-1):
                return self.result
        for value in (None, "{}", 0, bytearray(b"{}")):
            with self.subTest(value=repr(value)):
                with self.assertRaisesRegex(resource.InvalidInventory, "blocking binary"):
                    resource.load(InvalidReader(value))
        from unittest.mock import patch
        with patch.object(resource, "MAX_INPUT_BYTES", 2):
            with self.assertRaisesRegex(resource.InvalidInventory, "supplied byte budget"):
                resource.load(InvalidReader(b"{}  "))


if __name__ == "__main__":
    unittest.main()
