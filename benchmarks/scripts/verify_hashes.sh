#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
SPEC="$ROOT/benchmarks/benchmark-spec.yaml"
: "${GIT_BIN:?set GIT_BIN to an absolute git binary path}"

case "$GIT_BIN" in
    /*) ;;
    *) printf 'GIT_BIN must be an absolute path: %s\n' "$GIT_BIN" >&2; exit 1 ;;
esac
if [[ ! -x "$GIT_BIN" ]]; then
    printf 'GIT_BIN is not executable: %s\n' "$GIT_BIN" >&2
    exit 1
fi
if [[ ! -f "$SPEC" ]]; then
    printf 'benchmark spec not found: %s\n' "$SPEC" >&2
    exit 1
fi

parse_projects() {
    awk '
        /^source_evidence:/ { source = 1; next }
        source && /^scenarios:/ { exit }
        source && /^  projects:/ { projects = 1; next }
        projects && /^  - id:/ {
            if (id != "") print id "|" repo "|" commit "|" tag "|" raw "|" peeled
            id = $3; repo = commit = tag = raw = peeled = ""; next
        }
        projects && /^    repo:/ { repo = $2; next }
        projects && /^    commit:/ { commit = $2; next }
        projects && /^    tag:/ { tag = $2; next }
        projects && /^    raw_tag_object:/ { raw = $2; next }
        projects && /^    peeled_commit:/ { peeled = $2; next }
        END { if (projects && id != "") print id "|" repo "|" commit "|" tag "|" raw "|" peeled }
    ' "$SPEC"
}

while IFS='|' read -r id repo commit tag raw peeled; do
    if [[ -z "$id" || -z "$repo" ]]; then
        printf 'source project is missing id or repo\n' >&2
        exit 1
    fi
    if [[ "$repo" == local ]]; then
        if [[ -z "$commit" ]]; then
            printf 'local source is missing commit: %s\n' "$id" >&2
            exit 1
        fi
        actual=$("$GIT_BIN" -C "$ROOT" rev-parse --verify "${commit}^{commit}")
        if [[ "$actual" != "$commit" ]]; then
            printf 'local source commit mismatch for %s: expected %s, got %s\n' "$id" "$commit" "$actual" >&2
            exit 1
        fi
        continue
    fi

    if [[ -z "$tag" || -z "$raw" || -z "$peeled" ]]; then
        printf 'remote source is missing tag evidence: %s\n' "$id" >&2
        exit 1
    fi
    remote_refs=$("$GIT_BIN" ls-remote "$repo" "refs/tags/$tag" "refs/tags/$tag^{}")
    actual_raw=$(printf '%s\n' "$remote_refs" | awk -v ref="refs/tags/$tag" '$2 == ref { print $1 }')
    actual_peeled=$(printf '%s\n' "$remote_refs" | awk -v ref="refs/tags/$tag^{}" '$2 == ref { print $1 }')
    if [[ "$actual_raw" != "$raw" ]]; then
        printf 'remote tag object mismatch for %s: expected %s, got %s\n' "$id" "$raw" "$actual_raw" >&2
        exit 1
    fi
    if [[ "$actual_peeled" != "$peeled" ]]; then
        printf 'remote peeled commit mismatch for %s: expected %s, got %s\n' "$id" "$peeled" "$actual_peeled" >&2
        exit 1
    fi
done < <(parse_projects)
