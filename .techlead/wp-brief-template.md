# WP dispatch template — Docker-parity campaign (lead's calibration)

Every WP agent gets: its WP row (from the synthesis ledger) + the frozen root(s)
it binds to (parity-contract.md §0) + these invariants. Refined from real
freeze-gate + WP-LIFE-01 rework.

## Hard requirements (bake into every brief)

1. **TRANSCRIBE, don't design** (AP-3). Ambiguity → minimal Docker-faithful
   choice + note it in the return card. Never invent interfaces.

2. **Touch ONLY the WP's owned files.** No CLI/handler/shared-enum edits unless
   the WP explicitly owns them. (Shared surfaces get a lead-authored freeze.)

3. **New module → re-export it.** If you add a `pub mod`/`pub fn` that other
   crates/WPs consume, add the `pub use` in the crate's `lib.rs` (match the
   existing convention, e.g. `pub use run::mount::{...}`). Otherwise pub-in-
   private-module items are DEAD-CODE and fail `clippy -D`. No `#[allow]`.

4. **Tests MUST be parallel-safe.** CI runs `cargo test --workspace`
   MULTI-THREADED. NEVER mutate process-global state (`std::env::set_var`,
   cwd, global singletons) in tests. INJECT roots/paths as params (house
   convention — see `network::registry` taking `home: &Path`); each test uses
   its own tempdir (atomic-counter + nanos unique). A test needing
   `--test-threads=1` is a defect.

5. **Verify with the REAL gate, not `cargo check`.** Run (capped `-j2`, box is
   shared with the owner), IN ORDER:
   a. `cargo fmt --all` then `cargo fmt --all -- --check` (MUST be clean — the
      ubuntu CI gate runs fmt-check FIRST; a single mis-format fails the whole PR).
   b. `cargo clippy -p <crate> --all-targets -j2 -- -D warnings` (clippy, not check).
   c. `cargo test -p <crate> -j2` (NO `--test-threads=1` — mirror CI's parallel run).
   d. **If the WP changes CLI behavior, exit codes, stderr messages, flag
      surface, or any cross-cutting/end-to-end behavior, ALSO run
      `cargo test -p lightr-acceptance -j2`.** The acceptance suite codifies
      end-to-end CLI contracts (exit codes, messages) that per-crate tests miss.
      (Real miss: WP-EXIT-CODE ran `-p lightr-cli` only → broke
      `g4::a9b_unknown_ids`, which asserted the pre-parity exit 2 / "unknown
      run id". The self-hosted gate runs `cargo test --workspace`, so it WILL
      catch this — but only after a full runner cycle. Catch it locally.)
      **Acceptance tests exec the BUILT `lightr` bin (assert_cmd /
      CARGO_BIN_EXE_lightr), so always `cargo build --workspace` BEFORE
      `cargo test --workspace` locally** — a stale bin yields false failures
      (the gate builds first; mirror it).
   All three must be clean/green. `cargo check` passes on fmt + warning issues
   that are hard CI errors — never trust it as the gate.

6. **Invariants:** behavior-preserving on shipped paths; fail-closed (honest
   exit/error, never silent); tense-law (no unmeasured number as measured);
   <400 LOC/file; no debt/`#[allow]`/`--no-verify`.
   - **The godfile guard counts TOTAL lines of each .rs file, INCLUDING the
     `#[cfg(test)]` module.** "Production is under 400" does NOT pass. If your
     edits push a file's TOTAL over 400, you MUST split the test module into
     `<name>_tests.rs` via `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`
     (house convention — see network_tests.rs / imgmeta_tests.rs). No exceptions.

7. **Commit** to the worktree's CURRENT branch: subject imperative + scoped,
   why-body, trailer `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.

8. **Cross-platform: the windows CI gate is build+clippy (no tests).** If you
   import a `#[cfg(unix)]`/`#[cfg(windows)]`-gated module (e.g.
   `lightr_run::network` is unix-only — check the target crate's `lib.rs` for
   `#[cfg(...)]` before importing), your code WON'T compile on the other platform
   and the windows gate fails. cfg-gate your module/usage AND provide a
   fail-closed fallback (honest stub) for the unsupported platform at the
   dispatch/call site. Local macOS gates do NOT catch this — only windows CI does.
   8a. **`cfg(unix)` dead-code on windows (recurring trap — DF-06, STATS-TOP).**
   ANY fn / helper / test that is reachable ONLY from a `#[cfg(unix)]` path (e.g.
   a `ps`/`libc` parser called only by a `#[cfg(unix)]` query fn) is DEAD CODE on
   windows → `clippy -D` fails the windows gate. Put `#[cfg(unix)]` on the fn
   ITSELF (and on its tests / their `#[cfg(all(test, unix))] mod`), not just on
   the caller. Likewise, a binding used only inside a `#[cfg(unix)]` block is an
   unused-var on windows — gate the `let` with `#[cfg(unix)]` too. You run on
   macOS so you CANNOT see this; mentally compile for windows before finishing.

## Return card (entire return value — compact, not prose)
`<WP-id> | branch <name> @ <sha> | files: <list> | clippy -p <crate> -D: <pass/fail> | tests: <N passed, parallel> | ambiguities: <none|...> | LOC max`

## Lead integration (my side, per WP)
- Cold-check the diff vs spec (AP-5) — never trust the card.
- Re-gate myself ONLY if the box is free; else rely on PR CI (GitHub-side, zero
  box load) — push to the PR branch, merge ONLY on green (main never red).
- PR per taproot / grouped per axis for dependents. Delete branch + worktree on
  merge. Sync main. Keep the repo impeccable.
