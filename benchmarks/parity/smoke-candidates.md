# Smoke Candidates From Current Source

Status: candidate ledger. Candidate 1 is promoted as `build-single-stage` after
the exact fixture bytes were checked in at `benchmarks/fixtures/scratch-copy`
and pinned to commit `aa8be4981cfcf0856729a362a60c586580474f31`.
Candidates 2 and 3 remain unpromoted until runner setup/cleanup semantics are
implemented and tested.

## Shared Fixture

`crates/lightr-acceptance/tests/acceptance_r3/group_a25.rs:21-27` creates a
temporary context containing exactly:

```text
Dockerfile: FROM scratch\nCOPY data.txt /data.txt\n
data.txt:   a25
```

`$fixture` below means that exact temporary context. `$home`, `$dest`, `$tar`,
`$docker_tar`, and `$lightr_tar` are distinct paths in a harness-created temp
directory. `LIGHTR_HOME=$home` applies to every Lightr command.

## 1. Scratch COPY Build

**Docker command**

```sh
docker build --tag lightr-smoke-a25:local --file "$fixture/Dockerfile" "$fixture"
```

**Lightr command**

```sh
LIGHTR_HOME="$home" lightr build --name @t/d --file "$fixture/Dockerfile" "$fixture"
```

**Assertions**

```yaml
- kind: exit_code
  expected: 0
  scope: docker_and_lightr
- kind: command
  command: LIGHTR_HOME="$home" lightr hydrate "$dest" --name @t/d && cmp -- "$fixture/data.txt" "$dest/data.txt"
  expected: 0
```

**Platform/engine:** Docker Engine `28.3.2`; Lightr default `native` build
engine on macOS or Linux. No `RUN` instruction, so no host-shell or container
engine dependency.

**Cleanup**

```sh
docker image rm --force lightr-smoke-a25:local
LIGHTR_HOME="$home" lightr oci rmi @t/d
rm -rf "$fixture" "$dest" "$home"
```

**Source proof:** Clap declares `build <context>`, `-f/--file`, and
`-t/--name` in `crates/lightr-cli/src/cli/cmd/mod.rs:312-344`; dispatch reaches
`handlers::build::run` in `crates/lightr-cli/src/cli/dispatch.rs:277-313`;
handler resolves supplied Dockerfile and executes build in
`crates/lightr-cli/src/handlers/build.rs:61-105`. A25 executes same fixture
through Lightr's Docker-compatible build surface at
`crates/lightr-acceptance/tests/acceptance_r3/group_a25.rs:21-52`.

## 2. Docker-Save Tar Import

**Fixture setup:** run candidate 1 setup/build first, then create exact Docker
save input:

```sh
docker image save --output "$tar" lightr-smoke-a25:local
```

**Docker command**

```sh
docker image load --input "$tar"
```

**Lightr command**

```sh
LIGHTR_HOME="$home" lightr oci import "$tar" --name @t/import
```

**Assertions**

```yaml
- kind: exit_code
  expected: 0
  scope: docker_and_lightr
- kind: command
  command: LIGHTR_HOME="$home" lightr hydrate "$dest" --name @t/import && cmp -- "$fixture/data.txt" "$dest/data.txt"
  expected: 0
```

**Platform/engine:** Docker Engine `28.3.2`; Lightr store/import path on macOS
or Linux. No execution engine needed.

**Cleanup**

```sh
docker image rm --force lightr-smoke-a25:local
LIGHTR_HOME="$home" lightr oci rmi @t/import
rm -rf "$fixture" "$dest" "$tar" "$home"
```

**Source proof:** Clap declares `oci import <path> --name <ref>` in
`crates/lightr-cli/src/cli/cmd/subcommands.rs:99-115`; dispatch reaches
`handlers::oci::import` in `crates/lightr-cli/src/cli/dispatch.rs:168-184`;
handler accepts layout-directory or tar input in
`crates/lightr-cli/src/handlers/oci.rs:66-87`. Docker-save tar import and
hydrated-file assertion are current coverage in
`crates/lightr-oci/src/oci/tests/import_tests.rs:158-231`.

## 3. OCI Save Interoperability

**Fixture setup:** run candidate 2 setup/import. `@t/import` is then a Lightr
ref retaining the Docker-save source record.

**Docker command**

```sh
docker image save --output "$docker_tar" lightr-smoke-a25:local
```

**Lightr command**

```sh
LIGHTR_HOME="$home" lightr oci save @t/import --output "$lightr_tar"
```

**Assertions**

```yaml
- kind: exit_code
  expected: 0
  scope: docker_and_lightr
- kind: command
  command: docker image load --input "$lightr_tar"
  expected: 0
```

**Platform/engine:** Docker Engine `28.3.2`; Lightr store/export path on macOS
or Linux. No execution engine needed.

**Cleanup**

```sh
docker image rm --force lightr-smoke-a25:local
LIGHTR_HOME="$home" lightr oci rmi @t/import
rm -rf "$fixture" "$tar" "$docker_tar" "$lightr_tar" "$home"
```

**Source proof:** Clap declares `oci save <store-ref> -o/--output` in
`crates/lightr-cli/src/cli/cmd/subcommands.rs:130-143`; dispatch reaches
`handlers::oci::save` in `crates/lightr-cli/src/cli/dispatch.rs:174-180`;
handler documents Docker-save semantics and writes only to explicit output in
`crates/lightr-cli/src/handlers/oci_imageops.rs:78-112`. Faithful retained-record
save coverage is in `crates/lightr-oci/src/oci/tests/save_tests.rs:24-91`.

## `bench-compare` Excluded

Do not use `lightr bench-compare` for these runner scenarios. Its real CLI
spawns Docker only under `ProbePolicy::Spawn`
(`crates/lightr-cli/src/handlers/bench_compare/mod.rs:207-257`), but its build
probe intentionally uses Docker `FROM alpine` while Lightr uses `FROM scratch`
(`crates/lightr-cli/src/handlers/bench_compete_docker/probes.rs:51-95` and
`crates/lightr-cli/src/handlers/bench_compare/measure.rs:214-228`). Its output
is median timing rows, not per-scenario command/assertion evidence required by
the benchmark runner.
