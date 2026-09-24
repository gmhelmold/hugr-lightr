# SI-00 closure — accepted and integrated

**Date:** 2026-09-17. **Campaign:** #152. **Work package:** #153. **Delivery:** PR #154.
**Decision:** SI-00 and its explicitly scoped A07 integration prerequisites are accepted and integrated. The issue may now close as completed. SI-01–SI-05 and campaign qualification remain separate, uncompleted obligations.
**Authority/reviewer:** ChatGPT as author/coordinator under the owner's explicit execution and continuation-through-closure instructions. This is not a fabricated independent human/agent approval or a separate owner line-by-line review.

## Exact identities and integration

| Identity | Value |
|---|---|
| Technical plan | v2.2, `e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d` |
| Reviewed source head | `b558303d71a2df15397beb06213ba22d4479d596` |
| Actual tested PR merge checkout | `65196f369e102274af66bee8d797b65a685fa402` |
| Tested checkout parents | `19afb88d94db1d79cfe2a56ce7a99eab84f6abb3`, `b558303d71a2df15397beb06213ba22d4479d596` |
| Tested and integrated tree | `e6a62cd4f41b731c76d0936a58ef518a9204ab84` |
| Execution-input fingerprint | `dd5b44159e398e41225be0b72853872d285cb9cd7cfe549ecbecade31110ed89` |
| **Foundation input / actual integration merge** | **`0c48886d2c82e671c33466ca0d5ff1640f90985d`** |
| Target | `fix/snapshot-integrity`, NOT `main` |

GitHub accepted the expected-head-checked PR #154 merge. Read-back confirmed the actual merge has the same tree and ordered parents as the tested checkout; merge metadata differs. The archive was reconstructed locally and its Git tree matched the API's expected tree exactly. This later receipt changes documentation only and is not relabeled as a new tested executable.

[Final coordinator review](https://github.com/gmhelmold/hugr-lightr/pull/154#pullrequestreview-5239232816) records the scope, evidence, limits and integration decision. ADR-0020 is now **Accepted as design**, with its explicit compatibility exception and [final contract review](../bootstrap/FINAL-CONTRACT-REVIEW.md). Runtime protocol qualification is not implied.

## Delivered scope, including actual Rust changes

The bootstrap delivers pinned native CI, evidence and numerical-comparison validators, immutable schema vectors, reproducible CLI fixtures, native capability probes, source-reviewed caller/root/resource maps, experiment registration and a reviewed compatibility/support contract.

[A07](../bootstrap/A07-INTEGRATION-PREREQUISITES.md) explicitly permits narrow existing-protocol integration repairs. `cas/mod.rs` now uses atomically reserved operation-owned temporary directories in `atomic_write`, `put_bytes` and `ingest_file`. The new private `owned_temp.rs` keeps the CoW payload initially nonexistent and does not recursively delete unknown children. `scan.rs` preserves literal Unix backslash filenames and exact symlink target text, rejects selected unrepresentable UTF-8 inputs and propagates readlink errors. Test files and the preexisting test-only hash example received the pinned formatter corrections.

Nine new Rust test methods cover forced allocation collision, distinct allocations, conservative cleanup, parallel public ingestion, concurrent metadata writing and raw-OS path/link fidelity. Five apply on Windows; all nine apply on Unix. APFS's native invalid-byte filename refusal is asserted separately from the always-executed conversion rejection. No skip or lint allow was introduced.

**Not implemented here:** readiness/requalification, new journal/history formats, fresh-capture semantics beyond the delimited path repair, unified leases, shared-cache locking, composite named-version publication/recovery, authoritative GC and active-reader protection. Those remain the existing SI-01–SI-05 obligations. Legacy unchecked Windows flush behavior and exists-only reuse have not been declared fixed. No new storage protocol is activated.

## Final executable evidence

| Run | Verified result |
|---|---|
| [35252210277 — integration](https://github.com/gmhelmold/hugr-lightr/actions/runs/35252210277) | Required formatting, workspace build, full workspace tests, all-targets Clippy with denied warnings and three compiling causal mutants all passed |
| [35252210279 — native/bootstrap](https://github.com/gmhelmold/hugr-lightr/actions/runs/35252210279) | Five native profiles, independent matrix validation, support controls, resource capability and full F0-F5 CLI sequence passed |
| [35252210302 — immutable contract](https://github.com/gmhelmold/hugr-lightr/actions/runs/35252210302) | 11 canonical checks passed; zero errors/failures/skips; expectations were not regenerated |

The separate benchmark-evidence workflow also reported success; its status is NOT used as a final campaign performance verdict.

| Native storage/index profile | Passing executions | Historical ignores | Formatting exit |
|---|---:|---:|---:|
| Linux x86_64 | 129 | 1 | 0 |
| Linux aarch64 | 129 | 1 | 0 |
| macOS Intel | 115 | 1 | 0 |
| macOS ARM | 115 | 1 | 0 |
| Windows x86_64 MSVC | 111 | 0 | 0 |

**599 passing native executions**, not 599 unique/new tests. Full workspace logs separately contain **1,660 passed, zero failed, one preexisting ignored, zero filtered**, across 35 suite summaries. These totals overlap and must not be added as unique coverage. The historical Unix ignore is `index::tests::r1_tests_gc::gc_reclaims_hard_killed_run_dir`; no new ignore or causal-suite exemption was added.

The exact candidate's **42 Python test methods** passed in CI and again locally. The 11 canonical schema checks were also rerun locally against the archived, hashed native test-only hash helper. The ten frozen vectors retain decoded SHA-256 `8e0a4b7c52f99e12a8ecf91a4bdc706b547f5524d8e7464d06972e0b5b3221df`; their 7,020 rejected strict prefixes are subcases, not runtime tests. The future Rust storage codec is still unimplemented.

Each of the three causal mutations compiled and then failed its exact named test with exit 101 and a one-failed-test summary: Unix filename rewriting, symlink-target rewriting, and recursive claiming of an existing temporary directory. Build errors, empty suites, watchdogs and unrelated failures do not count. The pristine candidate passed those tests before mutation; mutations were confined to a disposable source copy.

### Real CLI and resources

F0-F3/F5 completed 20 measured iterations plus three warmups each; F4 completed seven measured iterations plus one warmup. Total: **107 measured, 16 warmups and 369 successful snapshot/snapshot/hydrate commands**, with independent restored-tree equality checks. F5 now completes, including literal-backslash and symlink cases. Four additional traced minimal cases roundtrip exactly with sources unchanged.

The actual retained CLI SHA-256 is `87b03afc8560532a6b4cee009d6e99932993e44e106b5c5b5efbb184d280a0d2`. Logs, per-command observations, resource measurements and fixture identities are retained. New-store versus warmed-store measurements do not claim a forced-cold OS page cache, tail percentile or final no-regression verdict. F6 and final numerical budgets still require the correct SI-03 semantic reference under C10; this bootstrap establishes the executable registration/comparison process rather than fabricating that future budget.

All five native archives were downloaded and independently revalidated against the pinned expected identities, test policy and every internal artifact/binary hash. Integration, contract, CLI and resource archives were also hash-checked. Foreign native test binaries were NOT executed in the authoring container.

The current owned 32 MiB resource image reproduced ENOSPC/errno 28, preserved control bytes, removed only owned filler and unmounted. This is OS experiment capability, not future Lightr recovery. Native profile receipts retain actual link/permission/clone/copy support; Windows file sync is not directory-entry power-loss certification. Actual journal interruption, quota/inode recovery and final resource headroom tests remain E18/SI-05.

## All five axioms / 21 criterion acceptance accounting

The original numbered criterion text and all 123 campaign IDs are unchanged. This is evidence accounting, not a replacement specification.

| Criterion | Acceptance evidence / explicit boundary |
|---|---|
| SC00.1 | CONTRACTS, caller maps and FINAL-CONTRACT-REVIEW bind shared C01-C08/C12/C13 semantics to callers, errors, locks and ownership; no incompatible helper/publication result is left for parallel workers to invent. |
| SC00.2 | Five required native smoke profiles actually executed; every artifact set and named test was checked. |
| SC00.3 | Executable diagnostics/fixtures, incident dispositions, E01-E22 recipes, performance methodology and bounded frozen wire specimens are delivered. |
| SC00.4 | ADR-0020 and final coordinator review ratify tuple/decision/history, legacy, links, attribution, topology/resource domains, inert activation and pending pre-scan policies. |
| QS00.1 | Existing ADRs were reviewed; intended differences are explicit, including bounded control-metadata JSON. Design acceptance is recorded before future protocol implementation. |
| QS00.2 | CI is scoped and tokens read-only; raw failure evidence is preserved. A07 explicitly records the limited runtime-scope exception before its repairs. No unrelated feature or skip change. |
| QS00.3 | Capability claims use executed receipts. Compiler/action identities are pinned; source/binary provenance and resource limits are retained. |
| QS00.4 | No extra request ledger, daemon, reserve, hidden link encoding or new persistent protocol has been introduced. |
| CC00.1 | Direct source review and raw inventory cover the frozen baseline's 73 direct sites in 35 files: 70 relevant bindings and three homonyms. Final inventory includes 494 Rust files/1,749 conservative occurrences. This is reviewed source binding, not invented whole-program compiler proof. |
| CC00.2 | Snapshot/build, OCI sidecars and mutators, history/undo/bisect/MCP, materializers, AC and raw namespace consumers are assigned; shared-helper regressions run in the workspace suite. |
| CC00.3 | Platform receipts, seams, caller/root/domain maps, named-test policy, incident hypotheses and fixture hashes are published and reproducible. |
| CC00.4 | Permission/link/clone/fallback capabilities, exact-SHA nonzero tests, artefact validation, complete F0-F5 baseline and budget-registration/comparison mechanism are executable. |
| CC00.5 | Legacy raw/envelope specimens, native link classes, aliases, shared-cache/two-store domains, no-current pending scenarios and future F6 semantic reference are bound to accepted owners/oracles. The safe resource runbook and conservative headroom calculation are frozen; actual new-protocol resource measurements belong to E18, not an invented bootstrap guarantee. |
| DOD00.1 | Compatibility/design review is recorded; no mandatory native smoke platform is unavailable. Narrow integration gates are all passing. |
| DOD00.2 | Caller/test inventory is reviewed by the coordinator; wrong identities, empty suites, absent profiles and missing/corrupt evidence are rejected by executed controls. No external review is claimed. |
| DOD00.3 | PR #154 is merged with the same tested tree. Exact foundation input is `0c48886d2c82e671c33466ca0d5ff1640f90985d`; handoff is accepted. |
| DOD00.4 | V2-A01-A06 and integrated-review findings map to accepted policies and existing E17-E22/E08/E11/E15/E18 recipes. Bounds/support/migration entry are frozen. Account-Project setup remains separate from runtime acceptance. |
| INV00.1 | No credentials, sibling writes or fictitious agents; runners are CI jobs. |
| INV00.2 | Native observations, reference-model checks and future runtime obligations remain distinct. |
| INV00.3 | Shared design accepted before downstream dispatch; new protocol stays inactive. No force-push or real-store rollout. |
| INV00.4 | No retroactive legacy-history or request-identity certification; all uncertainty and unsupported capability boundaries stay explicit. |

## Handoff and remaining campaign scope

**SI-01 is READY for its inert foundation work** from the exact merge above. **Only SI-02's isolated portion is READY**; its production primitive integration/closure still depends on SI-01. No worker is started by this receipt. G-FOUNDATION and G-ACTIVATION remain NOT ESTABLISHED.

Original untraced macOS/F1/F3 ENOENT events remain historically unattributed beyond their evidence. Controlled shared-temporary collision and literal-path/link defects have direct reproduction and regression witnesses; later passing samples do not invent a unique historical cause. Existing incident records are retained. This is an explicit scoped disposition, not an unexplained current failure hidden by a rerun.

The campaign and PR #146 remain open. Future receipts must satisfy fresh capture, requalification, composite recovery/GC/readers and final C10/C11 qualification. No main merge, release, user-data migration or automated coding-agent session occurred. Project creation previously required account-level authorization and is not silently marked complete.

## Durable archive index

All listed raw files are preserved in the accompanying Library delivery, not only referenced by expiring GitHub links. GitHub copies expire on 2026-10-17. Archive hashes protect integrity; they do not fabricate signed independent attestation.

| Artifact ID | Kind | Archive SHA-256 |
|---|---|---|
| 10510855528 | Workspace integration/source/causal controls | `a126fae3fe5d5a698348b02626b5d364f9afb374385c05879487ba9c09e54751` |
| 10510590437 | Contract/source/hash helper | `f5acfcf52f51230cf5139a809405c9647ae377350f847f16e53f411eedd39f42` |
| 10511236168 | Full CLI baseline/diagnostics/binary | `ad20498c1e532a5407aa69cbaa626017a4dfced335591fe88581816c9488b88a` |
| 10511125621 | Caller inventory/resource probe | `c1b4569940589a82c94a591ddcb61ee083a233cb4d697d9498192ebfb643253a` |
| 10510395977 | Native matrix verdict | `7e0f04a36b9f2e954a0bc1a8c293fd78f8c3b3c4f5e5f27e65beb4ac5c11c5c6` |
| 10510126069 | Linux x86_64 native | `2fefbd338487d4cd8f71efd1a32982b7b0bd733279ad5d241148d0debe77ccb5` |
| 10509679409 | Linux aarch64 native | `d690b10bd69e9e067dd48a8f1499d27f571f4eeba14c86a71b0d2a9a38c5d541` |
| 10509722096 | macOS Intel native | `5e7178b412c24fab464dfb4720519982b47c473ae6bcd5e871b6ae5565490805` |
| 10511015716 | macOS ARM native | `626c82aea7d658ab39048b3f7d7d6a1b9864f71465fdd0239b39178fb40f7d47` |
| 10510091226 | Windows native | `2132a0a40dcda8b08fd478f698bb5250f95eaab95de9d4569ee7705b2a46a3ac` |
