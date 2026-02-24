#!/usr/bin/env bash
set -u

eval "$(sops -d secrets.yaml | yq -r '. | to_entries[] | "export \(.key)=\(.value|@sh)"')"

exec "$@"
