# Vendored Source Provenance

`crates/lightr-cri` is vendored from
`https://github.com/HumanGuardrail/lightr-cri.git` at immutable commit
`3cf0a0f1c81f69fc66a85002c3895d90e095d0af` (2026-07-02), tree
`7f3d97e6e6f140fe0398b090b71832206300aa0d`.

The nested workspace remains outside parent Cargo workspace. Its `Cargo.lock`
is committed and every parent CI build of its opt-in composition uses `--locked`.

This is an imported source base, not a byte-for-byte mirror: parent integration
may carry reviewed adaptations in this directory. Review those adaptations as
part of the parent diff; do not treat an upstream checkout as an equivalence
oracle.

To verify source before an update:

```sh
git clone https://github.com/HumanGuardrail/lightr-cri.git /tmp/lightr-cri
git -C /tmp/lightr-cri checkout --detach 3cf0a0f1c81f69fc66a85002c3895d90e095d0af
git -C /tmp/lightr-cri rev-parse HEAD^{tree}
```

The final command must print
`7f3d97e6e6f140fe0398b090b71832206300aa0d`. Review any subsequent parent
adaptation from its committed diff.
