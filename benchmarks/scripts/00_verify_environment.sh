#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
EXPECTED_DOCKER_VERSION=28.3.2

require_absolute_executable() {
    local path=$1
    local name=$2
    case "$path" in
        /*) ;;
        *) printf '%s must be an absolute path: %s\n' "$name" "$path" >&2; exit 1 ;;
    esac
    if [[ ! -x "$path" ]]; then
        printf '%s is not executable: %s\n' "$name" "$path" >&2
        exit 1
    fi
}

: "${DOCKER_BIN:?set DOCKER_BIN to an absolute Docker binary path}"
require_absolute_executable "$DOCKER_BIN" DOCKER_BIN

client_version=$("$DOCKER_BIN" version --format '{{.Client.Version}}')
server_version=$("$DOCKER_BIN" version --format '{{.Server.Version}}')

if [[ "$client_version" != "$EXPECTED_DOCKER_VERSION" ]]; then
    printf 'Docker client version must be %s, got %s\n' "$EXPECTED_DOCKER_VERSION" "$client_version" >&2
    exit 1
fi
if [[ "$server_version" != "$EXPECTED_DOCKER_VERSION" ]]; then
    printf 'Docker server version must be %s, got %s\n' "$EXPECTED_DOCKER_VERSION" "$server_version" >&2
    exit 1
fi

printf 'docker_client=%s\n' "$client_version"
printf 'docker_server=%s\n' "$server_version"
