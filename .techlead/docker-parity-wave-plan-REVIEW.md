# EXHAUSTIVE PLAN REVIEW — Docker Parity Completion (v0.2)

**Review Date:** 2026-09-11 | **Baseline:** 2e41808
**Reviewer:** TechLead (self-review)

---

## 📋 REVIEW METHODOLOGY

Cross-referenced:
1. **parity-audit.md** — every 🟡/⏳/Staged item
2. **Actual codebase** — crate structures, existing handlers, shim coverage
3. **Docker CLI surface** — `docker --help` vs `lightr docker --help`
4. **TechLead invariants** — disjointness, contracts, no-debt, sweet-spot

---

## 🔍 FINDINGS BY CATEGORY

### **A. PARITY-AUDIT GAPS → PLAN COVERAGE**

| F-ID | Feature | Audit Status | Plan WP | Coverage |
|------|---------|--------------|---------|----------|
| F-304 | Networking (DNS/VPN) | 🟡 | WP-03 | ⚠️ Partial — plan says "DNS, VPN" but audit shows WP-NET1/2 only cover port-forward |
| F-308 | Restart Windows | 🟡 | WP-08 | ❌ **MISSING** — WP-08 is validation only, no Task Scheduler implementation |
| F-403 | Deep-memo nitro | 🟡 | — | ❌ **MISSING** — not in plan |
| F-603 | Microwave floor | 🟡 | WP-08 | ⚠️ Partial — validation only, not constrained HW measurement |
| F-604 | brew/curl/gh-releases | 🟡 | — | ❌ **MISSING** — distribution, not parity |
| **Staged: full networking (DNS/VPN)** | | 🟡 | WP-03 | ⚠️ Partial |
| **Staged: resource limits** | | — | WP-07 | ✅ Covered (GPU/Device/Caps) |
| **Staged: registry push** | | ✅ | — | Done (F-302) |
| **Staged: Rosetta** | | ⏳ | — | ❌ Out of scope (future ring) |
| **Staged: agent profiles** | | — | — | ❌ Not in F-ids |
| **Staged: deep-memo nitro shim** | | 🟡 | — | ❌ Same as F-403 |
| **Staged: healthcheck/secrets** | | ✅ (F-309) | — | Done |
| **Staged: restart-via-OS** | | 🟡 (Windows) | WP-08 | ❌ Implementation missing |

**VERDICT:** 4 of 7 🟡/Staged parity gaps are **not addressed** in plan.

---

### **B. DOCKER CLI SHIM GAPS (Critical for Drop-in Replacement)**

**Current shim supports (mod.rs:258-272):** `build|run|pull|images|ps|inspect|compose`

**Missing shim translations (Docker has 40+ subcommands):**

| Docker Command | Lightr Native | In Shim? | Plan Coverage |
|----------------|---------------|----------|---------------|
| `network` | ✅ `lightr network` | ❌ | WP-05 |
| `volume` | ✅ `lightr volume` | ❌ | WP-05 |
| `system` | ✅ `lightr system` | ❌ | WP-05 |
| `context` | ❌ | ❌ | WP-06 |
| `scan` | ❌ | ❌ | WP-06 |
| `login/logout` | ❌ | ❌ | ❌ **MISSING** |
| `save/load` | `oci import/export` | ❌ | ❌ **MISSING** |
| `tag` | ✅ `lightr tag` | ❌ | ❌ **MISSING** |
| `rmi` | ✅ `lightr rmi` | ❌ | ❌ **MISSING** |
| `history` | ✅ `lightr history` | ❌ | ❌ **MISSING** |
| `commit` | ✅ `lightr commit` | ❌ | ❌ **MISSING** |
| `version` | ✅ `lightr version` | ❌ | ❌ **MISSING** |
| `info` | ✅ `lightr info` | ❌ | ❌ **MISSING** |
| `exec` | ✅ `lightr exec` | ❌ | ❌ **MISSING** |
| `stop` | ✅ `lightr stop` | ❌ | ❌ **MISSING** |
| `start` | ✅ `lightr start` | ❌ | ❌ **MISSING** |
| `restart` | ✅ `lightr restart` | ❌ | ❌ **MISSING** |
| `kill` | ✅ `lightr kill` | ❌ | ❌ **MISSING** |
| `rm` | ✅ `lightr rm` | ❌ | ❌ **MISSING** |
| `cp` | ✅ `lightr cp` | ❌ | ❌ **MISSING** |
| `stats` | ✅ `lightr stats` | ❌ | ❌ **MISSING** |
| `top` | ✅ `lightr top` | ❌ | ❌ **MISSING** |
| `wait` | ✅ `lightr wait` | ❌ | ❌ **MISSING** |
| `rename` | ✅ `lightr rename` | ❌ | ❌ **MISSING** |
| `port` | ✅ `lightr port` | ❌ | ❌ **MISSING** |
| `pause/unpause` | ✅ `lightr pause/unpause` | ❌ | ❌ **MISSING** |
| `logs` | ✅ `lightr logs` | ❌ | ❌ **MISSING** |
| `attach` | ✅ `lightr attach` | ❌ | ❌ **MISSING** |

**TOTAL: 28 missing shim translations** — **WP-05 is severely undersized** (6 files → actually needs ~30 files across docker/ handler)

---

### **C. DOCKER FLAG GAPS (Per Subcommand)**

#### `docker build` (mod.rs:67-137) — **Already supports:** `-t`, `-f`, `--build-arg`, `--target`
| Flag | Status | Plan |
|------|--------|------|
| `--cache-from` | ❌ | WP-01 |
| `--secret` | ❌ | WP-01 |
| `--ssh` | ❌ | WP-01 |
| `--platform` | ❌ | ❌ **MISSING** |
| `--output` | ❌ | ❌ **MISSING** |
| `--no-cache` | ❌ | ❌ **MISSING** |
| `--pull` | ❌ | ❌ **MISSING** |

#### `docker run` (run_args.rs) — **Parses:** `-d`, `-p`, `-v`, `-e`, `--name`, `--rm`, `-it`, `-w`, `--user`, `--entrypoint`, `--env-file`, `--network`, `--restart`, `--cpus`, `--memory`, `--pids-limit`, `--gpus`, `--device`, `--cap-add`, `--cap-drop`, `--privileged`, `--read-only`, `--shm-size`, `--ulimit`, `--security-opt`, `--device-read-bps`, `--device-write-bps`, `--device-read-iops`, `--device-write-iops`
| Flag | Status | Plan |
|------|--------|------|
| `--gpus` | ❌ (parsed?) | WP-07 |
| `--device` | ❌ | WP-07 |
| `--cap-add/--cap-drop` | ❌ | WP-07 |
| `--privileged` | ❌ (F-204: honest exit-2) | ❌ **MISSING** |
| `--read-only` | ❌ (F-204: native honest Err) | ❌ **MISSING** |
| `--shm-size` | ❌ (F-204: native honest Err) | ❌ **MISSING** |
| `--platform` | ❌ | ❌ **MISSING** |
| `--init` | ❌ | ❌ **MISSING** |
| `--ipc` | ❌ | ❌ **MISSING** |
| `--pid` | ❌ | ❌ **MISSING** |
| `--uts` | ❌ | ❌ **MISSING** |
| `--cgroupns` | ❌ | ❌ **MISSING** |

#### `docker compose up` (compose.rs:42-107) — **Supports:** `-f`, `-p`, `--profile`, `--eager`, `--ttl`, `-d`
| Flag | Status | Plan |
|------|--------|------|
| `--build` | ❌ (honest exit-2) | WP-02 |
| `--remove-orphans` | ❌ | WP-02 |
| `--volumes` | ❌ | WP-02 |
| `--force-recreate` | ❌ | ❌ **MISSING** |
| `--no-recreate` | ❌ | ❌ **MISSING** |
| `--no-build` | ❌ | ❌ **MISSING** |
| `--abort-on-container-exit` | ❌ | ❌ **MISSING** |
| Service filter (positional) | ❌ (honest exit-2) | WP-02 |

#### `docker compose down` (compose.rs:110-145)
| Flag | Status | Plan |
|------|--------|------|
| `--remove-orphans` | ❌ | WP-02 |
| `--volumes` / `-v` | ❌ | WP-02 |
| `--rmi` | ❌ | ❌ **MISSING** |

---

### **D. ARCHITECTURAL DISJOINTNESS VIOLATIONS**

| WP Pair | Conflict | Resolution Needed |
|---------|----------|-------------------|
| WP-01 (BuildKit) × WP-02 (Compose+) | Both in `lightr-build/src/build/` — compose uses build lowering | **CONFLICT** — `build/compose/build_run.rs` calls `build::exec::run_build` |
| WP-03 (Network) × WP-04 (Volumes) | Both in `lightr-run` but different modules | ✅ OK — network in `network/`, volumes in `lightr-store/volume/` + `handlers/volume.rs` |
| WP-05 (CLI Shim) × ALL | Touches `docker/mod.rs` + `handlers/docker/` | **SEQUENTIAL BOTTLENECK** — WP-05 must wait for WP-03,04,06 contracts |

**CRITICAL:** WP-01 and WP-02 **cannot run in parallel** — they share `lightr-build/src/build/mod.rs` re-exports and `build_run.rs` calls into build exec.

---

### **E. MISSING WPs FROM PLAN**

| Missing Feature | Parity-Audit Ref | Effort | Suggested WP |
|-----------------|------------------|--------|--------------|
| Windows restart (Task Scheduler) | F-308 🟡 | Medium | WP-09 |
| `--privileged`, `--read-only`, `--shm-size` native/vz | F-204, F-203 | Medium | WP-10 |
| Multi-arch manifest list push | F-302 (partial) | Medium | WP-11 |
| AppArmor support (critest 80 skips) | F-CRI-CRITEST | Large | WP-12 |
| `--gpus`, `--device`, `--cap-add` run flags | F-203, F-204 | Medium | WP-07 (expand) |
| DNS/VPN (F-304 🟡) | F-304 🟡 | Large | WP-03 (expand) |
| Deep-memo nitro shim | F-403 🟡 | Large | WP-13 |
| Microwave floor measurement | F-603 🟡 | Medium | WP-08 (expand) |
| Windows restart (Task Scheduler) | F-308 🟡 | Medium | WP-09 |
| Multi-arch manifest list push | F-302 | Medium | WP-11 |
| `docker login/logout` | — | Small | WP-05 |
| `docker save/load` shim | — | Small | WP-05 |
| 28 missing shim subcommands | — | Large | WP-05 (split) |
| `--privileged`, `--read-only`, `--shm-size` native | F-203, F-204 | Medium | WP-10 |
| AppArmor (critest skips) | F-CRI-CRITEST | Large | WP-12 |

**Total missing WPs: 11 additional** — Plan has 8, reality needs ~19.

---

### **F. CONTRACT GAPS**

| Contract | Status | Issue |
|----------|--------|-------|
| C-01: CLI Flag Surface | Planned | ✅ Needed |
| C-02: Docker Shim Translation | Planned | ✅ Needed — but incomplete (28 missing) |
| C-03: Network/Volume Runtime | Planned | ✅ Partial — DNS/VPN missing |
| C-04: BuildKit Extensions | Planned | ✅ Partial — missing cache-from/secret/ssh |
| C-05: GPU/Device/Caps | Planned | ✅ Needed |
| **C-06: Resource Limits (native/vz)** | **MISSING** | Need `--privileged`, `--read-only`, `--shm-size`, `--pids-limit`, `--cpus`, `--memory` for native/vz |
| **C-07: Windows Restart** | **MISSING** | Task Scheduler integration |
| **C-08: Multi-arch Push** | **MISSING** | Manifest list synthesis |
| **C-09: AppArmor Profile** | **MISSING** | critest conformance |
| **C-10: Deep-memo Nitro** | **MISSING** | Nitro shim interface |

---

### **G. TIMELINE REALITY CHECK**

| Plan Estimate | Reality | Delta |
|---------------|---------|-------|
| Lead prep | 2 days | 5 days (11 contracts + scaffold) |
| WP-01..07 parallel | 5-7 days | 3-4 weeks (19 WPs, sequential bottlenecks) |
| WP-05 integration | 1 day | 1 week (28 shim subcommands) |
| WP-08 platform | 3 days | 2 weeks (owner HW + CI) |
| Integration + merge | 2 days | 1 week |
| **Total** | **~12 days** | **~6-8 weeks** |

**Primary bottleneck:** WP-05 (CLI Shim) is sequential after WP-03,04,06 and has 28 subcommands.

---

### **H. TECHLEAD INVARIANT VIOLATIONS**

| Invariant | Violation |
|-----------|-----------|
| **Disjointness** | WP-01 × WP-02 share `lightr-build/src/build/` |
| **Sweet-spot** | WP-05 (28 subcommands) >> 5 files / 400 LOC |
| **No debt** | 4 🟡 gaps unaddressed, 11 missing WPs = implicit debt |
| **Contracts frozen before dispatch** | Only 5 contracts, 11 missing |
| **Return-shape enforced** | WP-05 too large for single return card |

---

## 🎯 REVISED RECOMMENDATIONS

### **Option 1: Scope Down to "v0.2 Minimum Viable Parity" (4 weeks)**
Focus only on what blocks `docker compose up` / `docker build` / `docker run` drop-in:

| Priority | Scope | WPs |
|----------|-------|-----|
| **P0** | Build flags (`--cache-from`, `--secret`, `--ssh`) | WP-01 |
| **P0** | Compose flags (`--build`, `--remove-orphans`, service filter) | WP-02 |
| **P0** | Docker shim: add `network`, `volume`, `system`, `tag`, `rmi`, `history`, `commit`, `version`, `info`, `exec`, `stop`, `start`, `restart`, `kill`, `rm`, `cp`, `stats`, `top`, `wait`, `rename`, `port`, `pause`, `logs`, `attach` | WP-05 (split into 3) |
| **P0** | Network shim (`docker network` → `lightr network`) | WP-03 |
| **P0** | Volume shim (`docker volume` → `lightr volume`) | WP-04 |
| **P0** | System shim (`docker system` → `lightr system`) | WP-06 |
| **P1** | `--privileged`, `--read-only`, `--shm-size` for vz/ns | WP-10 |
| **P1** | Windows restart (Task Scheduler) | WP-09 |
| **P2** | `--gpus`, `--device`, `--cap-add` | WP-07 |
| **P2** | Multi-arch push | WP-11 |

### **Option 2: Full Parity (8-10 weeks)**
All 19 WPs, proper contracts, sequential integration.

---

## 📝 MY VERDICT

**PLAN REJECTED — Needs Major Revision**

**Reasons:**
1. **Severely under-scoped** — 8 WPs vs 19 needed; 28 missing shim translations
2. **Architectural conflict** — WP-01 × WP-02 not disjoint
3. **Sweet-spot violations** — WP-05 is 5× oversized
4. **4 🟡 gaps unaddressed** — implicit debt
4. **Timeline 5× optimistic** — 12 days vs 6-8 weeks
5. **11 missing WPs** — plan covers <50% of actual gaps

---

## 🔧 REQUIRED REVISIONS BEFORE APPROVAL

1. **Re-slice WPs** for true disjointness (split WP-01/02, split WP-05)
2. **Add 11 missing WPs** to plan table
3. **Add 6 missing contracts** (C-06 through C-11)
4. **Fix timeline** to 6-8 weeks with critical path
5. **Scope decision** — v0.2 MVP vs Full Parity (owner decision)
6. **Explicit waivers** for any 🟡 gaps deferred to v0.3

---

**Lead Signature:** _________________ **Date:** __________

**Owner Decision Required:** [ ] v0.2 MVP (4 weeks)  [ ] Full Parity (8-10 weeks)  [ ] Other: _______