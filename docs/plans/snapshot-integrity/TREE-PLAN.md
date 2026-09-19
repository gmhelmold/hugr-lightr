# SI-01 logical-tree preflight before output effects

Date: 2026-09-18. Part of #147 / #152; integration #146.
Base: `c6576ce6d4e06634d150a79f1987efaf085fa3da` (#167).
Authority: accepted ADR-0020 and C07/C12 of v2.2 at
`e30811971f6a1c2a1e2fdb0a91dc8ccf9a06879d`.

## Boundary

`TreePlan::validate` examines a borrowed, already decoded `Manifest` using
`TreeLimits` and cooperative `Wait` checks. It returns a complete structural
plan or an error. There is no filesystem I/O, payload read, native pathname
normalization, Store initialization, output allocation, symlink resolution,
new codec, runtime dependency or public routing change.

The plan borrows the original entries and path/target text; callers cannot
mutate the manifest while continuing to use it. It adds a deduplicated list
of explicit and implied directories, sorted so parents precede descendants.
Entry order is preserved in `entries()`; validation is independent of that order.
The LMF1 encoder/decoder and all original topology/destination tests are unchanged.

A plan is NOT a verified payload, native representation check, destination
inspection, namespace exclusion, empty-output check, Store/resource lease or
write authority. In particular, differently cased or canonically equivalent
Unicode names can be structurally distinct but collide on an actual filesystem.
Unix backslashes and colons remain literal logical characters; Windows adapters
must separately reject unsupported native names and derive valid link kinds.
Links retain target text (including dangling, absolute or parent-relative text)
without lookup. No link is created or followed by this pass.

Later output integration must combine whole-tree structural validation, actual
filesystem representability, protected-root preflight and anchored use under the
correct lifetime protection. None can be substituted for the others. The old
public hydrate path has NOT been changed or retrospectively declared safe.

## Checks and resource contract

Checks: version 1, u32 wire entry count, nonempty u16-byte-bounded UTF-8 entry
paths, no NUL, no empty/dot/parent path components, no leading/trailing separator,
nonempty u16-byte-bounded NUL-free link text, file permission bits within 07777,
checked u64 sum of file lengths equal to the manifest total, unique explicit
paths, and no file/link occupying any required ancestor directory.

These are LMF1/logical constraints, not a claim that a 65535-byte component is
creatable on a particular filesystem. A valid String is already UTF-8. No case
folding, Unicode normalization or native Path::components conversion occurs.

TreeLimits is caller supplied, with no guessed universal default. It caps entry
count, component occurrences across every entry (including repeated prefixes),
and path/target UTF-8 bytes. Zero permits an empty tree. All scalar and budget
checks precede allocation of two borrowed-reference indices. The directory
candidate count includes implied ancestors before deduplication. Both indices
use try_reserve_exact; a reported capacity/allocation failure becomes an explicit
OutOfMemory error. This does not guarantee process survival under arbitrary
allocator failure, cap memory used by earlier decoding, or promise exact RSS.

Malformed tree data yields InvalidData; exceeded caller work budgets yield
InvalidInput. Checkpoint errors retain their original kind/message and are not
retried. Checks occur between bounded passes, entries and components, including
before a plan is returned. Standard sorting is not asynchronously interruptible.
The work is bounded by caller entry/component/text limits; no file bytes are read.

## Five acceptance groups

**Success criteria:** reject a structurally inconsistent whole tree before
returning a plan; retain exact logical names, targets and input order; include
all implied directories; no output effect or native capability is fabricated.

**Completeness criteria:** empty/unsorted trees; duplicate kinds; file/link
ancestors in all entry orders; shared directories and sibling-prefix controls;
normal-component rules, UTF-8 wire lengths, literal backslashes/case/Unicode;
link text, mode/version, totals/overflow; all caller budgets and every checkpoint;
independent small-tree oracle; borrow lifetime; preservation of earlier tests.

**Quality standards:** pinned Rust 1.96.0 and denied warnings; no dependency,
unsafe block, syscall, decoder or format change; bounded fallible indices; all
new logical tests required on every native platform; scoped in-memory fixtures;
causal controls must compile and fail the exact expected assertion.

**Invariants:** no name rewriting, invented live source, partial-plan success,
output creation, payload-readiness assumption, implicit native representation,
foreign resource authority, waived prior criterion or protocol activation.

**Definition of Done:** full exact-candidate CI plus native/bootstrap/causal
workflows pass; reviewed source/required-name identities verified; explicitly
labeled coordinator self-review; expected-head merge to integration only; new
parent #146 CI checked; only the completed delivery branch retired. This
increment cannot establish full C07/C12, G-FOUNDATION or G-ACTIVATION.

## Executable verification

Fifteen platform-independent Rust methods, plus one positive and one negative
borrow-lifetime doctest. The pairwise reference oracle checks 144 small trees
inside one method, not 144 additional tests or filesystem experiments. Five
Python policy methods include 45 absent/ignored/failed synthetic witness cases;
these check evidence acceptance, not the Rust validator itself.

Six compiled controls remove duplicate, ancestor-kind, total, budget, normal
component or implied-directory checks. The harness preserves every result,
requires the exact named assertion, and rejects timeout/compilation failure as
evidence. Pristine and restored fifteen-test selections must both pass.

```sh
cargo +1.96.0 test --locked -p lightr-store --lib store::foundation::tree_plan::
cargo +1.96.0 test --locked -p lightr-store --doc
cargo +1.96.0 clippy --locked -p lightr-store --all-targets -- -D warnings
python3 -m unittest discover -s scripts/si00 -p 'test_*.py' -v
python3 scripts/si01/tree_plan_controls.py --out /absolute/new/evidence-directory
```

Rust outcomes and exact head/tree/run identities belong to the live delivery
review, not an assertion that writing this document has qualified the code.
Public writer activation and native representation remain later obligations.
`production_protocol_enabled=false`; #147/#152 remain open and #146 draft.
