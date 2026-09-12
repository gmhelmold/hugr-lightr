# Benchmark Corpus Manifest

Generate detached manifest:

```sh
./benchmarks/scripts/generate_manifest.sh
```

Artifact identity: `benchmarks/MANIFEST.sha256`.
It contains SHA-256 records for sorted explicit corpus allowlist. It excludes
itself, generated/build/work/result paths, and this mutable explanation.
