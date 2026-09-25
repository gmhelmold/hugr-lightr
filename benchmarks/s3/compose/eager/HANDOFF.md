# S3 Eager Compose Handoff

This directory contains one runner-compatible S3 case. Its only shared
assertion is command exit. The current runner observes only that value.

No readiness, timing, teardown, resource-cleanup, logs, dependency-ordering,
health, or project-isolation assertion is declared here. Those observations do
not exist in current `bench-runner run-s3`; naming them would be false evidence.

## S3-6 Dependencies

S3-6 owns `benchmarks/runner/**` and `.github/workflows/benchmark.yml`. It must
land all items below before this case can claim eager compose semantics:

1. Extend S3 scenario schema with per-tool readiness and teardown argv plus
   declared namespaced resource kinds.
2. Start timing immediately before each tool command; stop timing only after
   identical readiness argv succeeds. Record command-return time separately.
3. Run teardown for both tools on every outcome, then prove declared resources
   absent after teardown. Serialize host-port cases.
4. Observe and emit real assertions for ports, dependency order, selected
   profiles, project isolation, logs, and configured health state. Do not map
   every assertion ID to command exit.
5. Add CI invocation for this exact spec and its mutation tests. Current
   benchmark CI validates runner surfaces but does not execute this eager spec.

Until then, this case is schema-validated only. It is not readiness evidence.
