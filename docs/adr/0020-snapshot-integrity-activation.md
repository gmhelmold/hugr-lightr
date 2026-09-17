# ADR-0020 — Snapshot integrity: bounded metadata and coordinated activation

- **Status:** Accepted as design on 2026-09-17 by the coordinator under the owner's continuation-through-closure mandate; see the review record below. This is not runtime qualification.
- **Date:** 2026-09-17
- **Scope:** campaign #152, bootstrap #153, PR #154. No protocol is activated by this document.
- **Contract:** execution specification v2.2 at `e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d`.

## Accepted compatibility decisions

ADR-0009 remains the content-plane foundation. ADR-0010's stat-only identity
remains a status optimization but is superseded for new-protocol snapshot publication:
a stable source must be represented by this capture's bytes. ADR-0017's
Windows symlink-copy fallback is superseded for new-protocol exact materialization by
preserve-or-reject representation/capability classes. Native validation is a
mandatory campaign qualification gate rather than a postponed runbook claim.
ADR-0012 requires actual numerical comparison; an echo-only job is not a gate.
ADR-0004's reference grammar and LMF1/current RefRecord encodings are retained.

The current RefRecord has **no magic prefix**: it begins with a little-endian
u16 name length. Do not confuse it with the 72-byte LRR1 Action Cache record.
New envelopes must have a tag disjoint from *semantically valid* legacy names,
not merely from the prefix of an unrelated record type.

## Selected bounded wire contract

Only metadata is framed; payload CAS and LMF1 are unchanged. Bounded control-record
JSON is an explicit exception to ADR-0009's border-only JSON rule, not a JSON
manifest or payload format. Every frame is
`8-byte tag | u32-LE body length | exact UTF-8 JSON body | 32-byte BLAKE3`.
The checksum covers tag, length and body with the domain
`lightr/snapshot-integrity/metadata/v1/`. It detects accidental damage, not an
adversary with store write access. Reject duplicate fields, unknown required
schema versions, invalid UTF-8, non-integer integer fields, trailing bytes,
unchecked lengths and allocation before length validation. JSON has a fixed
field order on write; recovery stores/reuses exact bytes, not a reserialization
that invents a new historical identity.

| Record | Tag bytes | Maximum JSON body | Required contents |
|---|---|---:|---|
| Readiness | `00 00 4c 53 49 52 30 31` | 4,096 bytes | version=1, digest, length u64, assurance profile |
| History | `00 00 4c 53 49 48 30 31` | 196,608 bytes | version=1, exact RefRecord base64, two optional digest pointers, provenance |
| Pending | `00 00 4c 53 49 50 30 31` | 1,048,576 bytes | version=1, operation ID/kind, ref name/key, PREPARED/COMMIT_DECIDED, before/after tuples, slot, expected envelope, retirement identity |

The leading zero name length cannot represent an accepted reference name.
Legacy parsing must validate grammar and full consumption; successful decoding
alone cannot establish legacy provenance. RefRecord bytes are bounded at
131,147 (two maximum u16 strings plus fixed fields); their base64 representation
is bounded at 174,864 bytes. Reject overlong strings before invoking the
existing truncating encoder. Bounds are engineering choices, not measurements.
Golden vectors, all truncation offsets and boundary/duplicate-key cases must
precede acceptance of the future Rust codec. No codec is implemented here.

Digest strings are exactly 64 lower-case hex characters; tuple pointers are
either null or one digest, never filesystem paths. Operation IDs are fixed
128-bit random identifiers used only while pending exists; absence after a lost
reply yields AttributionUnknown. There is no request-dedup ledger. Numeric
slots use checked u64 arithmetic, maximum numeric slot plus one and exclusive
allocation. All three tuple components are explicitly present, including null.

## Resource domains and lifetime

Store identity is the opened/canonical root's native filesystem identity;
aliases must converge. Store-private scratch belongs under a reserved managed
staging directory on that store's filesystem. The shared index has its own
stable lock at the canonical index root; a Store lock cannot authorize cleaning
that index. Preserve the shared index location, including standalone callers.

Order: Store lease -> optional cache EX section -> ref locks sorted by key ->
digest locks sorted by key. Cache is a leaf section and must be released before
ref/digest acquisition. Standalone cache callers never acquire a Store lease
while holding the cache lock. No lock upgrades or recursive wrapper calls.
Unknown overlapping authoritative roots are rejected, not merged by spelling.
Lock files survive unlock and are never classified as temporary data.

## Public seam and activation

The implementation seam is detailed in
`../plans/snapshot-integrity/bootstrap/CONTRACTS.md`. SI-01 adds inert primitives
and tests; G-FOUNDATION explicitly reports `production_protocol_enabled=false`.
SI-02 tests real preparation through controlled fixtures, without changing
public routing. Ordered SI-03 slices convert codecs/decision, producers,
readers/history and collectors together. Only the final slice may establish
G-ACTIVATION on disposable stores. A runtime feature flag may not silently opt a
partial binary into the protocol. Old-path defects remain defects while inert.

G-ACTIVATION is not SI-05 qualification and not approval to migrate user data.
Mixed old/new writers and unqualified downgrades are unsupported. No new daemon,
transaction database, emergency-space reserve, sibling-repository mutation or
additional runtime dependency is selected. Existing old binaries cannot be
made to honor a new marker retroactively; quiescence and compatible-client
rollout are explicit operational preconditions, not a claimed access control.

## Acceptance record

Reviewer and decision-maker: ChatGPT, author/coordinator, under the owner's
explicit `OK pode executar`, `pode seguir ate fechar` and `siga` mandate.
No independent human, agent or owner line-by-line review is represented.
[Final contract/compatibility review](../plans/snapshot-integrity/bootstrap/FINAL-CONTRACT-REVIEW.md)
ratifies the schema, baseline caller/resource map, error/support surface and
G-FOUNDATION/G-ACTIVATION boundary. The review preserves all five package axioms.

The ten frozen native-authored vectors have decoded SHA-256
`8e0a4b7c52f99e12a8ecf91a4bdc706b547f5524d8e7464d06972e0b5b3221df`.
Checks use the locked native hash helper and independent reference conformance;
normal CI does not regenerate expectations. This proves specification-oracle
conformance, not a production Rust codec or legal runtime state transitions.

Design acceptance is complete. SI-00 handoff is still conditional on its final
reviewed integration receipt; this ADR alone does not mark a package DONE,
start SI-01, establish G-FOUNDATION, activate the protocol, migrate real data,
merge main or authorize release. A07's narrow existing-protocol repairs remain
explicitly separate from implementing this new protocol.
