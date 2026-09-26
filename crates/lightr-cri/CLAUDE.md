# CLAUDE.md

Context for AI agents working in this repo. Keep it lean + high-signal.

## What lightr-cri is

**A stateless, crash-only CRI implementation backed by CoreLink CAS via
Lightr's crates.** The kubelet-facing shell of the Lightr runtime: it will
ship as the `lightr-cri` crate / `lightr cri serve` opt-in server mode. Built
HERE as a nested workspace inside hugr-lightr, against the seam contract.

```
kubelet ── CRI gRPC (unix socket) ──▶ lightr-cri shell (THIS REPO)
                                        │ CriBackend trait (FROZEN seam)
                              ┌─────────┴─────────┐
                        fake backend         lightr backend
                        (in-memory, R0)      (hugr-lightr crates, integration)
```

## Principles (decided — don't relitigate without the owner)

1. **The listener owns nothing.** Kernel + disk are the source of truth;
   the listener is a stateless view. `kill -9` it mid-operation and nothing
   is lost, nothing reconciles — there was never a mirror.
2. **Scoped no-daemon.** Lightr's "no daemon, ever" holds for local use;
   `cri serve` is an explicit opt-in server mode that honors the spirit:
   stateless, crash-only, socket-activatable, zero owned state.
3. **Contract-first.** All work codes against `docs/contract/` (transcribed
   types + CriBackend semantics). The nested workspace stays independent from
   the core workspace; the opt-in composition root owns integration.
4. **Conformance is the acceptance suite.** critest/crictl items are the
   red→green anchor; GREENLIST no-regression contract (pattern from
   skill-004-kubernetes-production).
5. **Linux validates; macOS only compiles.** CNI, namespaces, critest run in
   CI on Linux runners. Anything not exercised on Linux is probe-truthful
   gated, never silently claimed (Lightr law).
6. **Fail closed; tense discipline.** No measured claims until a bench/CI
   run signs them. Targets cite precedents.
7. **Not a crusade.** We do not chase managed-k8s conversion (GKE/AKS slots
   are closed). ICP: HuGR's own Runners fabric first, self-managed k8s as a
   CoreLink door second.

## Relationship to the rest of HuGR

- This workspace is vendored under `hugr-lightr/crates/lightr-cri`; the parent
  repository owns composition and unified Git history.
- `lightr-cri-serve` depends on this nested workspace by path and exposes the
  opt-in integration without pulling tonic/gRPC into the core workspace.
- CoreLink/clw semantics are upstream law, inherited through Lightr.

## Conventions

- English for repo documents; lean, evidence-cited (house style).
- Rust 2021, toolchain pinned to match hugr-lightr (1.96.0); deps
  exact-pinned where the house pins. gRPC: tonic/prost (this workspace
  introduces them; they stay behind the opt-in CRI composition root).
- Commits: `Co-Authored-By` trailers; canonical author gustavo@humangr.com.
- Branch → PR → merge; gates green before merge. Build waves via TechLead
  (`.techlead/` installed): decompose → contract → pack → dispatch → verify.

## Don't touch

- Sibling repos under `~/Documents/HuGR/` have other live sessions —
  read-only there, always.
- `docs/contract/` changes require explicit owner sign-off (it is a freeze).
- `.techlead/` is gitignored session state — never commit its contents.
