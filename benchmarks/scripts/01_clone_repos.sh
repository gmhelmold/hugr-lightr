#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
SPEC="$ROOT/benchmarks/benchmark-spec.yaml"
REPOS_DIR="$ROOT/benchmarks/work/repos"
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

require_absolute_executable "$GIT_BIN"
if [[ ! -f "$SPEC" ]]; then
    printf 'benchmark spec not found: %s\n' "$SPEC" >&2
    exit 1
fi

bash "$ROOT/benchmarks/scripts/verify_hashes.sh"
mkdir -p "$REPOS_DIR"

while IFS='|' read -r id repo commit tag raw peeled; do
    if [[ "$repo" == local ]]; then
        continue
    fi
    if [[ -z "$id" || -z "$tag" || -z "$peeled" ]]; then
        printf 'remote source is missing id, tag, or peeled commit\n' >&2
        exit 1
    fi

    destination="$REPOS_DIR/$id"
    if [[ -e "$destination" ]]; then
        if [[ ! -d "$destination/.git" ]]; then
            printf 'repository destination is not a git worktree: %s\n' "$destination" >&2
            exit 1
        fi
        actual=$("$GIT_BIN" -C "$destination" rev-parse HEAD)
        if [[ "$actual" != "$peeled" ]]; then
            printf 'existing repository commit mismatch for %s: expected %s, got %s\n' "$id" "$peeled" "$actual" >&2
            exit 1
        fi
        continue
    fi

    "$GIT_BIN" clone --no-checkout "$repo" "$destination"
    "$GIT_BIN" -C "$destination" fetch --depth 1 origin "refs/tags/$tag"
    "$GIT_BIN" -C "$destination" checkout --detach "$peeled"
    actual=$("$GIT_BIN" -C "$destination" rev-parse HEAD)
    if [[ "$actual" != "$peeled" ]]; then
        printf 'cloned repository commit mismatch for %s: expected %s, got %s\n' "$id" "$peeled" "$actual" >&2
        exit 1
    fi
done < <(parse_projects)
