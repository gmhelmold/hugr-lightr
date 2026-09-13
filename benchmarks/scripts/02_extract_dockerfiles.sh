#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
SPEC="$ROOT/benchmarks/benchmark-spec.yaml"
: "${GIT_BIN:?set GIT_BIN to an absolute git binary path}"

require_absolute_executable() {
    case "$1" in
        /*) ;;
        *) printf 'GIT_BIN must be an absolute path: %s\n' "$1" >&2; exit 1 ;;
    esac
    if [[ ! -x "$1" ]]; then
        printf 'GIT_BIN is not executable: %s\n' "$1" >&2
        exit 1
    fi
}

local_project_commit() {
    local wanted_project=$1
    awk -v wanted="$wanted_project" '
        /^source_evidence:/ { source = 1; next }
        source && /^scenarios:/ { exit }
        source && /^  projects:/ { projects = 1; next }
        projects && /^  - id:/ { id = $3; repo = commit = ""; next }
        projects && /^    repo:/ { repo = $2; next }
        projects && /^    commit:/ {
            commit = $2
            if (id == wanted && repo == "local") print commit
        }
    ' "$SPEC"
}

supported_fixtures() {
    awk '
        /^scenarios:/ { scenarios = 1; next }
        scenarios && /^- id:/ {
            if (id != "" && availability == "supported") print project "|" path
            id = $3; availability = project = path = ""; fixture = 0; next
        }
        scenarios && /^  availability:/ { availability = $2; next }
        scenarios && /^  fixture:/ { fixture = 1; next }
        fixture && /^    project:/ { project = $2; next }
        fixture && /^    path:/ { path = $2; next }
        END { if (id != "" && availability == "supported") print project "|" path }
    ' "$SPEC"
}

require_absolute_executable "$GIT_BIN"
if [[ ! -f "$SPEC" ]]; then
    printf 'benchmark spec not found: %s\n' "$SPEC" >&2
    exit 1
fi

while IFS='|' read -r project path; do
    if [[ -z "$project" || -z "$path" || "$path" == null ]]; then
        printf 'supported scenario has no fixture project or path\n' >&2
        exit 1
    fi
    commit=$(local_project_commit "$project")
    if [[ -z "$commit" ]]; then
        printf 'supported fixture project must be local in S1: %s\n' "$project" >&2
        exit 1
    fi
    "$GIT_BIN" -C "$ROOT" rev-parse --verify "${commit}^{commit}" >/dev/null
    object_type=$("$GIT_BIN" -C "$ROOT" cat-file -t "${commit}:${path}")
    if [[ "$object_type" != tree ]]; then
        printf 'fixture path is not a tree at pinned commit: %s:%s\n' "$commit" "$path" >&2
        exit 1
    fi
done < <(supported_fixtures)

# bench-runner alone materializes verified fixtures with git archive.
