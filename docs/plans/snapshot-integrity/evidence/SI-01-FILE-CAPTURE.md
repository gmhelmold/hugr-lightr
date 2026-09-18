# SI-01: handle-based live-file capture

Date: 2026-09-18. Parent WP #147; campaign #152; integration PR #146.
Status: local candidate, pending exact-head remote gates and reviewed integration.
This receipt does not establish G-FOUNDATION or G-ACTIVATION.

## Source and scope

Implementation source: `03607ade258fd2333c9b67da7d8bc6ec5e1f6d8e`.
Base: `31c16fc84dde3af206982b4e149133f17a211465` (#163 already integrated inspection-witness policy).
Contract: accepted ADR-0020, C03 live-source staging; all five SI-01 axiom
groups and 21 criteria remain mandatory. `production_protocol_enabled=false`.

`LeasedStagedFile::capture_file` consumes an already-open, caller-validated
regular live file. It attempts a native clone or performs an explicitly forced
ordinary copy. The result reports the actual native rung, or byte-copy plus
its unsupported-clone cause. An empty capture reports byte copy, not an
unexecuted native capability. No source pathname is reopened; no source data,
permissions or attributes are modified. The caller transfers cursor ownership;
fallback reads from offset zero. Concurrent writes through external source
aliases are not claimed to be an application-wide atomic snapshot.

Both paths use the EXISTING verified staging finalizer: bounded 64 KiB hash,
actual length/digest checks, checked writable-handle sync, and rewind. A failed
clone reservation must be cleaned before a new exclusive copy allocation.
Unknown scratch blocks cleanup/fallback instead of being recursively removed.
Space/quota, permission, interruption and other I/O failures are not rebranded
as unsupported capabilities. Only the explicit native unsupported-error list
permits one fallback, retaining that error in the successful method report.

The native operations are macOS fclonefileat, Linux FICLONE, and Windows block
clone with bounded per-request extents/checkpoints. A returned OS success is
still followed by staged-byte verification. Windows outputs are newly created
read/write files, not files inheriting the live source's read-only attribute.
macOS clone mode is adjusted only on the owned output handle. Native call sites
have safety comments and were self-reviewed for live handle/pointer lifetimes.

Primary interface references:
- https://github.com/apple-oss-distributions/xnu/blob/main/bsd/man/man2/clonefile.2
- https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ns-winioctl-duplicate_extents_data
- https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/1f7ddf4d-9302-42b9-8374-0aecb0f2569a

The public Store/CLI routes and `publication.rs` are byte-identical to the base.
In particular, **unconfirmed CAS requalification stays an ordinary full rewrite,
never CoW**. This new primitive is for live inputs only and does not implement
C12 public-path preflight, resource reaping or typed metadata caller conversion.
A staged result remains borrowed from its Store lease and is not a readiness
proof; the existing checked publisher must finish before issuing PreparedObject.

## Five acceptance groups for this increment

**Success criteria:** independently captured bytes survive later source changes;
copy fallback removes a partial clone tail; real resource/I/O failure cannot
become successful copy fallback or issue a proof.

**Completeness criteria:** cover zero/multi-buffer files, real native outcome,
forced copy, unsupported partial clone, false clone success, cancellation,
flush failure, read-only source, pathname replacement after open, unrecognized
scratch and composition with existing publication and ordinary requalification.

**Quality standards:** shared verification, bounded buffers and loops, native
error preservation, scoped non-global seams, pinned Rust 1.96.0, no source file
over 400 lines, no dependency/format/protection relaxation. Test-only fake clone
helpers are never reported as native filesystem witnesses.

**Invariants:** no live-source changes, hardlinks, unlink of published payloads,
existence-only readiness, changed requalification, public routing activation,
recursive unknown-scratch cleanup or lease-free proof.

**Definition of Done:** exact candidate passes local checks plus complete CI,
applicable five-platform/native/CLI and causal gates; labeled self-review,
expected-head integration, then fresh parent #146 CI. A local success or this
receipt alone cannot authorize merge. WP #147 and campaign #152 stay open.

## Local evidence

Owner's Intel x86_64 Mac, macOS 15.3.2; isolated worktree
`/tmp/lightr-si01-capture-wfxual_w/repo`. Rust 1.96.0, RUSTFLAGS=-D warnings,
two build jobs, debug info/incremental disabled to bound disk use. No host disk
was filled and no unrelated scratch/checkout was deleted.

- Full Store library: **188 passed, zero failed/ignored/filtered**.
- Store all-target Clippy and formatter: PASS.
- Store doctests: **3 passed**, including existing lease lifetime checks.
- SI-00 Python support/policy: **49 passed**; CI-policy: **13 passed**.
- Capture selection: **13 portable methods**. On this Mac the read-only-source
  native witness reports actual Native(Clone) and requires it, rather than
  silently counting copy fallback as the macOS clone witness.
- Four compiled causal defects: bypass verification, swallow resource failure,
  ignore sync failure, omit hash checkpoints. All compiled, exited 101 in the
  exact required assertion, then the restored 13-method selection passed.
  `scripts/si01/file_capture_controls.py` reproduces them in disposable source.

These counts overlap. No Linux or Windows native outcome is inferred from this
Mac run. Windows successful ReFS cloning still needs a ReFS witness; an NTFS copy
fallback is not that witness. Injected ENOSPC is a controlled error, not the
real constrained-resource E18 scenario. No final performance claim is made.

All 13 new names are registered as mandatory in native expected-tests.json;
the previous 90-name prefix and all other policies are preserved. Two additional
Python methods check unique registration/declarations and no added ignore.
The new controls job is additive; every prior CI job remains in place.

An initial local Clippy run rejected an unnecessary `drop(proof)` in a test.
The scoped borrow's natural last use replaces it; no lint suppression was added.
An intermediate registration sanity check rejected a count mismatch; duplicate
macOS-only coverage was consolidated into the portable native-outcome test,
which explicitly requires actual cloning on macOS. Final count is 13 on all
supported profiles; no old committed test was removed.

Raw local log SHA-256 values:

```json
{
  "store-tests-final.log": "dc2254207faea7686b584e9a42c2df92a616d32c42c882d1ecbab5011f2a0a9f",
  "store-clippy-final.log": "8033b4f48e9e5b40d691c57d5d1043605dcebb40b21fe648c3e6eabfcc12aea7",
  "store-doctests.log": "930b907188d21c781ab9eabeefbc84336f121a69e528922f742a591747ddef0a",
  "policy-tests.log": "b3d83147c7b5a59893b7a39d39541ec05862b1f9e4186a9c6b347bd7bb5836fe",
  "ci-policy-tests.log": "12ab8b2983a5efd9dd8142e0abb0e74ce6ffa742f606f26c32ff5d1026d94fab",
  "store-clippy.log": "23212420d56abee1310ccca8cf9c3fce140e17786856958595f40573b8deca29"
}
```

Causal receipt (hashes refer to the archived source and exact local child logs):

```json
{
  "schema": 1,
  "checkout": "03607ade258fd2333c9b67da7d8bc6ec5e1f6d8e",
  "tree": "2abe593398f14d55ad454a104af5bae746314926",
  "controls": [
    {
      "name": "verify",
      "build_exit": 0,
      "test_exit": 101,
      "test": "store::cas::preparation::file_capture::tests::file_capture_claimed_clone_success_still_checks_digest_and_length"
    },
    {
      "name": "resource",
      "build_exit": 0,
      "test_exit": 101,
      "test": "store::cas::preparation::file_capture::tests::file_capture_clone_resource_error_is_not_hidden_by_fallback"
    },
    {
      "name": "sync",
      "build_exit": 0,
      "test_exit": 101,
      "test": "store::cas::preparation::file_capture::tests::file_capture_sync_error_remains_fatal_for_both_capture_modes"
    },
    {
      "name": "hash-checkpoint",
      "build_exit": 0,
      "test_exit": 101,
      "test": "store::cas::preparation::file_capture::tests::file_capture_cancel_after_clone_and_during_hash_cleans_only_owned_file"
    }
  ],
  "production_protocol_enabled": false,
  "status": "CAPTURE_CONTROLS_PASSED",
  "artifacts": {
    "hash-checkpoint-build.log": "ea76c98ae99ffa9a6382462b0c0b9d8db27acf2b86b1c24680b5276860873790",
    "hash-checkpoint-test.log": "bc0b5ff681c91e52dc3bbaead14c488baf63c8e85da5841f152db34617649721",
    "pristine.log": "061966fd44ebc96863d81072952930d03f1764ddde222eae6efcd2bad873e979",
    "resource-build.log": "8ac88bc93c1fee901ed36fac62d76275048f47d460c2f47390e14b66d9d40075",
    "resource-test.log": "c68e4fb3c94d7531013aa5e17ba7c36dadc917aef8a1fe351393f68eac5619af",
    "restored.log": "c96f285d2600a1b8bc66030a1f609547d9b1dd3b362036f265a5af897e7b5dcf",
    "source.zip": "b8a2c7904d40caa735f0799948751a59ece5c2614e57539d754947c6fb848a5a",
    "sync-build.log": "d22cdd9e411cee17941c38a699c1fae65a0431a6254bf92309866cd6611c5a64",
    "sync-test.log": "c0a34b89055fa0d6d11981a4fe139f936a00ab8924e5970fea015711b4a3bf9f",
    "verify-build.log": "081824f8356405878e67eac3d1b928294fa96aaf6199e304f72ac3f303e19375",
    "verify-test.log": "67f3b50908f2a34b9e74c4b83dc79ffd0c103db85eabef6fcd64e95b2ce0aed3"
  }
}
```

Remote execution, eventual merge SHA and parent verification belong in the linked
PR/issue receipt after they actually occur. This later document is not a new
Rust executable identity. Historical failed logs are retained, not overwritten.
