# Immutable native-authored metadata vectors

The ten vectors were authored by the repository-locked native BLAKE3 helper in
run `35237459468`, artifact `10504230892`, source head `01d71d7`.
The exact UTF-8 JSON is stored using deterministic gzip only to avoid duplicating
large wire-hex fields. Decompression is bounded by `scripts/si00/goldens.py`:

- Exact decoded size: 24,177 bytes.
- SHA-256: `8e0a4b7c52f99e12a8ecf91a4bdc706b547f5524d8e7464d06972e0b5b3221df`.
- Ten frames, 7,020 strict truncation prefixes in total.

Normal CI no longer uses `--generate`. It reads these bytes, checks the pinned
identity and tests them against the locked native helper. The independent Python
BLAKE3 reference checks the same frames and nine official hash-vector prefixes.
The reference's source/license are retained under `scripts/si00/vendor/`.
Neither storage compression nor these test-only implementations changes a
production storage encoding, runtime dependency or accepted ADR status.

The UNDO/ADOPT prior-reference rule is checked using resigned negative frames;
it does not alter any of the original native-authored golden bytes. Replacing
expected answers requires an explicit reviewed fixture/contract change, not
regeneration during the verifying run.
