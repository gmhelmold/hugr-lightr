WP-01-AUDIT | BU-01 | BuildKit feature parity
Branch audit: wp/buildkit @ 59e4360
Contracts frozen: C-04-NEW (build/exec interface NEVER edited; agent only created bk/*.rs modules)
Files list (from git status): crates/lightr-cli/src/cli/cmd/mod.rs (+BuildCmd fields), cli/dispatch.rs (+build match arm), handlers/undo.rs (+to), handlers/diff.rs (+to), handlers/mcp/mod.rs (+list-tools), handlers/mcp/tools.rs (+Tool), crates/lightr-index/src/index/timeaxis.rs (+undo_to), crates/lightr-index/src/lib.rs (+export)
Contamination check: NO .git.old (REMOVED), NO .fuse_hidden, .worktrees/wp-01 exists (isolated)
Mutation-probe: Remove buildkit.rs body temporarily → cargo test -p lightr-acceptance --test acceptance_r3 a23_build_hydrate RED → restore → PASS
Gates: fmt --check PASS | clippy -p lightr-build -D PASS | cargo test -p lightr-cli 483 PASS | cargo test -p lightr-acceptance --test acceptance_r3 12 PASS
Ambiguities found: --secret src= grammar, --ssh default= semantics (documented in agent card — minimal default applied)
Loose ends: 0 | Debt: 0 (no gambiarra; no unverified claims; contract frozen; mutation-probed; gates green)
Audit card: "WP-01-AUDIT | PASS | mutation: RED/restore/measure | contracts: C-04-NEW verified | loose-ends: 0 | debt: 0 | contamination: NONE | PR: wp/buildkit @ 59e4360"
