#!/usr/bin/env bash
set -u
set -o pipefail

if ! export_lines="$(sops -d secrets.yaml | yq -r '. | to_entries[] | "export \(.key)=\(.value|@sh)"')"; then
  printf 'Failed to load Cloudflare environment from secrets.yaml\n' >&2
  exit 1
fi

if [ -z "$export_lines" ]; then
  printf 'No Cloudflare environment values were loaded from secrets.yaml\n' >&2
  exit 1
fi

eval "$export_lines"

exec "$@"
