# CLAUDE.md

Operational instructions for contributors. Keep changes lean, evidence-cited,
and consistent with accepted architecture decisions.

## Lightr

Lightr is a daemonless, imageless runtime. Workspaces materialize from a
content-addressed store, run under the lightest isolation appropriate to their
context, and reuse known action results before provisioning.

Read before changing behavior:

- `docs/whitepaper/hugr-lightr-v2.md`: canonical vision.
- `docs/adr/`: accepted architecture decisions. Write code only against
  accepted ADRs.
- `docs/spec/build-spec-v2.md`: frozen R0 surfaces and acceptance criteria.
- `docs/ARCHITECTURE.md`: engines and seams.
- `docs/MVP-v0.1.md`: scope and definition of done.

## Principles

1. **No daemon, ever**: nothing runs when no command runs; `ps` must prove it.
2. **No images**: use CAS manifests and chunks, lazy by default. OCI is an
   import format, not model.
3. **Free local, forever, no account**: local operation must not require login
   or a remote service.
4. **Isolation a la carte**: `native` provides reproducibility, not a sandbox.
   Use `fc` hardware boundaries for hostile tenancy.
5. **Memoize-first**: check action cache before provisioning.
6. **Pure client**: do not change server-side tenancy, auth, or dedup semantics.
7. **Fail closed**: verify pinned inputs before spawn; return explicit errors;
   never publish partial results.
8. **Never charge for customer's own compute twice**.

## Evidence And Claims

- Deduplication is intra-tenant at GA. Cross-tenant dedup remains staged;
  never describe it as live.
- Claim benchmark, performance, or footprint numbers only when measured on
  named reproducible hardware with a cited reproduce path. Otherwise label
  numbers as targets, not measurements.
- Keep claimed behavior aligned with code and documented acceptance criteria.

## Dependencies And Boundaries

- Lightr uses CAS, action-cache, workspace, and runner interfaces as a client.
  Do not fork or modify server-side behavior from this repository.
- Sibling repositories under `~/Documents/HuGR/` may have active work. Inspect
  them read-only if needed; never mutate them. Work only in this repository.
- Local operation must work without optional network bridges. Keep `net_fd`
  optional; `None` remains valid for store, run, CLI, and engine paths.
- Wire bridging and LAN mesh are independent optional capabilities. Do not make
  either a prerequisite for local operation.

## Conventions

- Write documentation in English, lean and evidence-cited.
- Follow repository commit conventions, including required trailers.
- Use Rust dependency pinning patterns already established in repository.
- Crate name is `hugr-lightr`; binary is `lightr`. Verify name availability
  before publication.
- Use branch, PR, and green required checks before merge.

## Do Not Touch

- Do not mutate sibling repositories.
- Do not change `docs/whitepaper/hugr-lightr-v2.md` section 13 principles or
  tense-discipline rules without explicit approval.
- Do not commit local agent/session state.
