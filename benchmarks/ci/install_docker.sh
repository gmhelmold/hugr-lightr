#!/usr/bin/env bash
set -euo pipefail

readonly DOCKER_VERSION=28.3.2
readonly DOCKER_PACKAGE_VERSION=5:28.3.2-1~ubuntu.24.04~noble

if [[ ! -r /etc/os-release ]]; then
    printf 'operating system metadata not readable: /etc/os-release\n' >&2
    exit 1
fi
os_codename=$(awk -F= '$1 == "VERSION_CODENAME" { gsub(/^"|"$/, "", $2); print $2; exit }' /etc/os-release)
if [[ $os_codename != noble ]]; then
    printf 'Docker pin requires Ubuntu noble\n' >&2
    exit 1
fi

sudo apt-get update
sudo apt-get install --yes --no-install-recommends ca-certificates curl
sudo install --mode=0755 --directory /etc/apt/keyrings
sudo curl --fail --show-error --silent --location \
    https://download.docker.com/linux/ubuntu/gpg \
    --output /etc/apt/keyrings/docker.asc
sudo chmod a+r /etc/apt/keyrings/docker.asc
printf '%s\n' \
    "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu noble stable" \
    | sudo tee /etc/apt/sources.list.d/docker.list >/dev/null
sudo apt-get update
sudo apt-get install --yes --no-install-recommends \
    "docker-ce=$DOCKER_PACKAGE_VERSION" \
    "docker-ce-cli=$DOCKER_PACKAGE_VERSION" \
    containerd.io docker-buildx-plugin docker-compose-plugin
sudo systemctl enable --now docker

if [[ ! -x /usr/bin/docker ]]; then
    printf 'Docker binary not executable: /usr/bin/docker\n' >&2
    exit 1
fi

client_version=$(sudo /usr/bin/docker version --format '{{.Client.Version}}')
server_version=$(sudo /usr/bin/docker version --format '{{.Server.Version}}')
if [[ $client_version != "$DOCKER_VERSION" ]]; then
    printf 'Docker client version must be %s, got %s\n' "$DOCKER_VERSION" "$client_version" >&2
    exit 1
fi
if [[ $server_version != "$DOCKER_VERSION" ]]; then
    printf 'Docker server version must be %s, got %s\n' "$DOCKER_VERSION" "$server_version" >&2
    exit 1
fi

sudo /usr/bin/docker info >/dev/null
sudo /usr/bin/docker version
