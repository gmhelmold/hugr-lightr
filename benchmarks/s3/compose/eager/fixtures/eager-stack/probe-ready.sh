#!/bin/sh
set -eu

# Shared readiness oracle. Runner invokes identical argv for Docker and Lightr;
# timer ends only after this command exits zero.
curl --fail --silent --show-error http://127.0.0.1:18080/ >/dev/null
