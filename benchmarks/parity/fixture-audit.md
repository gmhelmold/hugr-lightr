# Fixture Audit

## Frozen Source Evidence

| Project | Remote tag object | Peeled commit | Fixture result |
|---|---|---|---|
| `kubernetes-kind` | `5d985937e8a112a321916efd4ad3936c7db6345f` | `55019c83b0fd51ef4ced8c29eec2c4847f896e74` | No scenario fixture path/context declared or verified. |
| `elasticsearch` | `85ff3fe65dcf2ab0185083aee4b8f462a92ab289` | `da95df118650b55a500dcc181889ac35c6d8da7c` | No scenario fixture path/context declared or verified. |

Tags resolved with `git ls-remote <repo> refs/tags/<tag> refs/tags/<tag>^{}`.
`raw_tag_object` is evidence only. Fixture validation keys on `peeled_commit`.

## Kubernetes Project Mismatch

Current project ID `kubernetes-kind` points to `kubernetes/kubernetes`, not Kind.
Existing scenarios referenced Kind-oriented paths but supplied no verified fixture.
No scenario was silently moved to Kind.

Proposed follow-up, not applied to benchmark spec: inventory
`kubernetes-sigs/kind` at an owner-selected pinned tag, prove each path/context,
then add exact Docker and Lightr commands plus assertions before marking support.
