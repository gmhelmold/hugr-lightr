"""C13 native observations plus deterministic failure controls; not recovery tests."""
import errno
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

import test_resource_estimate as fixtures

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/si01/resource_capacity.py"
SPEC = importlib.util.spec_from_file_location("resource_capacity", SCRIPT)
capacity = importlib.util.module_from_spec(SPEC)
with mock.patch.dict(sys.modules, {"resource_estimate": fixtures.resource}):
    SPEC.loader.exec_module(capacity)


def raw(**changes):
    result = dict(f_bsize=4096, f_frsize=512, f_blocks=1000, f_bfree=900,
                  f_bavail=700, f_files=100, f_ffree=60, f_favail=50,
                  f_flag=0, f_namemax=255)
    return dict(result, **changes)


class FakeNative:
    """Synthetic descriptors only; never forwards close to the OS."""
    def __init__(self, *, errors=None, counters=None, mode=stat.S_IFDIR):
        self.errors = errors or {}
        self.raw = raw() if counters is None else counters
        self.mode = mode
        self.calls = []

    def call(self, name, argument=None):
        self.calls.append((name, argument))
        if name in self.errors:
            raise self.errors[name]

    def supported(self):
        return True

    def open(self, path):
        self.call("open", path)
        return 12345

    def metadata(self, fd):
        self.call("identity", fd)
        return SimpleNamespace(st_dev=7, st_ino=42, st_mode=self.mode)

    def counters(self, fd):
        self.call("statvfs", fd)
        return SimpleNamespace(**self.raw)

    def close(self, fd):
        self.call("close", fd)


class CapacityArithmeticTests(unittest.TestCase):
    def test_available_blocks_use_fragment_size_not_io_size_or_total_free(self):
        observed, unknown = capacity.decode_counters(raw())
        self.assertEqual(observed, dict(free_bytes=700 * 512, free_inodes=50, quota_bytes=None))
        self.assertEqual(unknown, ["quota_not_inspected"])

    def test_real_zero_capacity_is_preserved(self):
        observed, _ = capacity.decode_counters(raw(f_bavail=0, f_favail=0))
        self.assertEqual(observed["free_bytes"], 0)
        self.assertEqual(observed["free_inodes"], 0)

    def test_zero_inode_accounting_is_unknown_not_exhausted(self):
        observed, unknown = capacity.decode_counters(raw(f_files=0, f_ffree=0, f_favail=0))
        self.assertIsNone(observed["free_inodes"])
        self.assertIn("inode_accounting_unavailable", unknown)

    def test_counter_sentinels_never_become_unlimited_capacity(self):
        for sentinel in capacity.SENTINELS:
            with self.subTest(sentinel=sentinel):
                observed, _ = capacity.decode_counters(raw(f_bavail=sentinel, f_files=sentinel))
                self.assertIsNone(observed["free_bytes"])
                self.assertIsNone(observed["free_inodes"])

    def test_unavailable_fragment_size_has_no_guessed_fallback(self):
        for unit in (0, *capacity.SENTINELS):
            with self.subTest(unit=unit):
                observed, _ = capacity.decode_counters(raw(f_frsize=unit))
                self.assertIsNone(observed["free_bytes"])

    def test_invalid_native_counter_types_and_ranges_are_rejected(self):
        for number in (True, 1.5, "2", None, float("nan"), -2, capacity.U64_MAX + 1):
            with self.subTest(number=number), self.assertRaises(fixtures.resource.InvalidInventory):
                capacity.decode_counters(raw(f_bavail=number))

    def test_contradictory_counters_are_not_capacity(self):
        for changes in (dict(f_bavail=901), dict(f_bfree=1001),
                        dict(f_favail=61), dict(f_ffree=101)):
            with self.subTest(changes=changes), self.assertRaisesRegex(
                    fixtures.resource.InvalidInventory, "inconsistent"):
                capacity.decode_counters(raw(**changes))

    def test_byte_product_overflow_is_rejected(self):
        with self.assertRaisesRegex(fixtures.resource.InvalidInventory, "overflow"):
            capacity.decode_counters(raw(f_frsize=1 << 63, f_bavail=2))

    def test_fragment_size_is_not_an_allocation_profile(self):
        observed = capacity.observe_directory("fixture", native=FakeNative())
        self.assertEqual(observed["status"], "CAPACITY_OBSERVED")
        self.assertFalse(observed["allocation_profile_measured"])
        self.assertNotIn("profile", observed)

    def test_estimator_preserves_unknown_quota_in_composition(self):
        observed = capacity.observe_directory("fixture", native=FakeNative())
        inventory = fixtures.fixture()
        inventory["pools"][0]["capacity"] = observed["capacity"]
        result = fixtures.resource.estimate(inventory)
        self.assertEqual(result["status"], "INCOMPLETE_INFORMATION")
        self.assertEqual(result["exit_code"], 3)
        self.assertFalse(result["retry_authorized"])


class CapacityFailureTests(unittest.TestCase):
    def test_invalid_paths_fail_before_open(self):
        for path in ("", "nul\0path", "x" * 4097, None):
            native = FakeNative()
            with self.subTest(path=str(path)[:12]):
                result = capacity.observe_directory(path, native=native)
                self.assertEqual(result["status"], "OBSERVATION_FAILED")
                self.assertEqual(native.calls, [])

    def test_precancel_prevents_open(self):
        native = FakeNative()
        def stop():
            raise InterruptedError(errno.EINTR, "scoped cancellation")
        result = capacity.observe_directory("fixture", native=native, checkpoint=stop)
        self.assertEqual(result["errors"][0]["phase"], "checkpoint_before_open")
        self.assertEqual(result["errors"][0]["errno"], errno.EINTR)
        self.assertEqual(native.calls, [])

    def test_unsupported_platform_does_not_allocate(self):
        native = FakeNative()
        native.supported = lambda: False
        result = capacity.observe_directory("fixture", native=native)
        self.assertEqual(result["status"], "UNSUPPORTED_CAPABILITY")
        self.assertEqual(result["exit_code"], 3)
        self.assertEqual(native.calls, [])

    def test_open_failure_never_falls_back_to_parent(self):
        native = FakeNative(errors={"open": FileNotFoundError(errno.ENOENT, "missing")})
        result = capacity.observe_directory("missing/leaf", native=native)
        self.assertEqual(native.calls, [("open", "missing/leaf")])
        self.assertEqual(result["errors"][0]["errno"], errno.ENOENT)
        self.assertEqual(result["descriptor_cleanup"], "not_opened")

    def test_metadata_and_measurement_errors_close_once(self):
        for phase in ("identity", "statvfs"):
            with self.subTest(phase=phase):
                native = FakeNative(errors={phase: OSError(errno.EIO, "scoped native error")})
                result = capacity.observe_directory("fixture", native=native)
                self.assertEqual(result["errors"][0]["phase"], phase)
                self.assertEqual(result["errors"][0]["errno"], errno.EIO)
                self.assertIsNone(result["capacity"])
                self.assertEqual(native.calls.count(("close", 12345)), 1)

    def test_wrong_opened_type_is_not_a_directory_observation(self):
        native = FakeNative(mode=stat.S_IFREG)
        result = capacity.observe_directory("fixture", native=native)
        self.assertEqual(result["errors"][0]["errno"], errno.ENOTDIR)
        self.assertNotIn(("statvfs", 12345), native.calls)
        self.assertEqual(native.calls.count(("close", 12345)), 1)

    def test_cleanup_error_cannot_leave_an_accepted_capacity_projection(self):
        native = FakeNative(errors={"close": OSError(errno.EIO, "close uncertain")})
        result = capacity.observe_directory("fixture", native=native)
        self.assertEqual(result["status"], "OBSERVATION_FAILED")
        self.assertFalse(result["observation_accepted"])
        self.assertIsNone(result["capacity"])
        self.assertEqual(result["descriptor_cleanup"], "close_result_unknown")
        self.assertEqual(result["errors"][0]["phase"], "close")
        self.assertEqual(native.calls.count(("close", 12345)), 1)

    def test_initial_and_cleanup_errors_are_both_preserved(self):
        native = FakeNative(errors={"statvfs": OSError(errno.EIO, "measurement failed"),
                                    "close": OSError(errno.EBADF, "close failed")})
        result = capacity.observe_directory("fixture", native=native)
        self.assertEqual([e["phase"] for e in result["errors"]], ["statvfs", "close"])
        self.assertEqual([e["errno"] for e in result["errors"]], [errno.EIO, errno.EBADF])

    def test_post_measurement_and_post_close_cancellation_withhold_projection(self):
        for fail_at, phase in ((4, "checkpoint_after_measurement"), (5, "checkpoint_after_close")):
            native, calls = FakeNative(), []
            def checkpoint():
                calls.append(None)
                if len(calls) == fail_at:
                    raise TimeoutError(errno.ETIMEDOUT, "scoped deadline")
            with self.subTest(fail_at=fail_at):
                result = capacity.observe_directory("fixture", native=native, checkpoint=checkpoint)
                self.assertEqual(result["errors"][0]["phase"], phase)
                self.assertIsNone(result["capacity"])
                self.assertIn("raw_statvfs", result)
                self.assertEqual(native.calls.count(("close", 12345)), 1)

    def test_all_native_observations_use_the_same_opened_handle(self):
        native = FakeNative()
        result = capacity.observe_directory("alias/../target", native=native)
        self.assertEqual(native.calls, [("open", "alias/../target"), ("identity", 12345),
                                        ("statvfs", 12345), ("close", 12345)])
        self.assertTrue(result["observation_accepted"])
        for name in ("retry_authorized", "runtime_qualified", "production_protocol_enabled",
                     "pool_identity_verified", "pathname_binding_verified", "writability_verified",
                     "inventory_complete", "allocation_profile_measured", "quota_inspected"):
            self.assertIs(result[name], False, name)

    def test_malformed_native_reply_produces_valid_json_without_projection(self):
        result = capacity.observe_directory("fixture", native=FakeNative(counters=raw(f_bavail=float("nan"))))
        self.assertEqual(result["status"], "OBSERVATION_FAILED")
        self.assertIsNone(result["capacity"])
        json.dumps(result, allow_nan=False)


@unittest.skipUnless(capacity.Native().supported(), "native Linux/macOS observation only")
class CapacityNativeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def test_native_identity_counter_arithmetic_and_no_user_file_mutation(self):
        original = self.root / "keep"
        original.write_bytes(b"unchanged user bytes")
        before = original.stat()
        result = capacity.observe_directory(self.root)
        self.assertEqual(result["status"], "CAPACITY_OBSERVED", result)
        self.assertEqual(result["directory_identity"], dict(device=self.root.stat().st_dev,
                                                           inode=self.root.stat().st_ino))
        counters = result["raw_statvfs"]
        self.assertEqual(result["capacity"]["free_bytes"], counters["f_frsize"] * counters["f_bavail"])
        self.assertEqual(list(self.root.iterdir()), [original])
        self.assertEqual(original.read_bytes(), b"unchanged user bytes")
        self.assertEqual((original.stat().st_mtime_ns, original.stat().st_mode),
                         (before.st_mtime_ns, before.st_mode))

    def test_native_missing_directory_is_not_created_and_parent_is_not_measured(self):
        result = capacity.observe_directory(self.root / "not-created" / "nested")
        self.assertEqual(result["status"], "OBSERVATION_FAILED")
        self.assertEqual(result["errors"][0]["errno"], errno.ENOENT)
        self.assertEqual(list(self.root.iterdir()), [])
        self.assertNotIn("directory_identity", result)

    def test_native_regular_file_is_not_accepted(self):
        path = self.root / "file"
        path.write_bytes(b"untouched")
        result = capacity.observe_directory(path)
        self.assertEqual(result["status"], "OBSERVATION_FAILED")
        self.assertEqual(path.read_bytes(), b"untouched")

    def test_native_alias_parent_uses_kernel_resolution_not_lexical_collapse(self):
        (self.root / "actual" / "child").mkdir(parents=True)
        (self.root / "actual" / "target").mkdir()
        (self.root / "target").mkdir()
        (self.root / "alias").symlink_to(self.root / "actual" / "child", target_is_directory=True)
        path = self.root / "alias" / ".." / "target"
        result = capacity.observe_directory(path)
        self.assertEqual(result["status"], "CAPACITY_OBSERVED", result)
        self.assertEqual(result["directory_identity"]["inode"], (self.root / "actual" / "target").stat().st_ino)
        self.assertNotEqual(result["directory_identity"]["inode"], (self.root / "target").stat().st_ino)

    def test_native_replaced_path_does_not_change_the_opened_measurement_object(self):
        path, retained = self.root / "target", self.root / "retained"
        path.mkdir()
        expected = path.stat().st_ino
        class RenamingNative(capacity.Native):
            def open(self, original):
                fd = super().open(original)
                Path(original).rename(retained)
                Path(original).mkdir()
                return fd
        result = capacity.observe_directory(path, native=RenamingNative())
        self.assertEqual(result["status"], "CAPACITY_OBSERVED", result)
        self.assertEqual(result["directory_identity"]["inode"], expected)
        self.assertNotEqual(path.stat().st_ino, expected)
        self.assertFalse(result["pathname_binding_verified"])

    def test_native_owned_descriptor_is_closed_after_observation(self):
        class RecordingNative(capacity.Native):
            def open(self, path):
                self.fd = super().open(path)
                self.inheritable = os.get_inheritable(self.fd)
                return self.fd
        native = RecordingNative()
        result = capacity.observe_directory(self.root, native=native)
        self.assertEqual(result["descriptor_cleanup"], "closed")
        self.assertFalse(native.inheritable)
        with self.assertRaises(OSError) as caught:
            os.fstat(native.fd)
        self.assertEqual(caught.exception.errno, errno.EBADF)

    @unittest.skipUnless(sys.platform.startswith("linux"), "raw-byte filename is a Linux witness")
    def test_native_non_utf8_path_roundtrips_json_without_rewriting_name(self):
        path = os.fsencode(self.root) + b"/\xff"
        os.mkdir(path)
        result = capacity.observe_directory(path)
        encoded = json.dumps(result, ensure_ascii=True)
        self.assertEqual(result["status"], "CAPACITY_OBSERVED", result)
        self.assertEqual(os.fsencode(json.loads(encoded)["requested_path"]), path)

    def test_cli_emits_json_and_preserves_failure_exit(self):
        for path, code in ((self.root, 0), (self.root / "missing", 2)):
            command = [sys.executable, str(SCRIPT), "--directory", str(path)]
            process = subprocess.run(command, capture_output=True, text=True, timeout=10)
            with self.subTest(path=path):
                self.assertEqual(process.returncode, code, process.stderr)
                record = json.loads(process.stdout)
                self.assertEqual(record["exit_code"], code)
                self.assertFalse(record["retry_authorized"])
        self.assertEqual(list(self.root.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
