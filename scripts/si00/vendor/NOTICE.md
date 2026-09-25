# Test-only BLAKE3 reference

`pure_blake3.py` is an unmodified copy of `oconnor663/pure_python_blake3`'s
reference implementation, Git blob `5a9db753540be51c0d3307656299c02e2900eb6c`.
Source: https://github.com/oconnor663/pure_python_blake3/blob/main/pure_blake3.py
License: CC0-1.0, included in LICENSE-CC0. Source/license inspected 2026-09-17.

It is used only by Python conformance tests, independently of the repository's
locked Rust BLAKE3 implementation that originally generated the golden frames.
It is not a runtime dependency or a replacement for production cryptography.
Nine official BLAKE3 test-vector prefixes and segmented updates are checked in
`test_schema_conformance.py`. Upstream vectors:
https://github.com/BLAKE3-team/BLAKE3/blob/master/test_vectors/test_vectors.json
