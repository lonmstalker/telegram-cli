#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
env_file="$repo_root/.env.local"

if [ ! -f "$env_file" ]; then
  echo ".env.local is missing" >&2
  exit 2
fi

mode=$(stat -f '%Lp' "$env_file" 2>/dev/null || stat -c '%a' "$env_file" 2>/dev/null || true)
if [ "$mode" != "600" ]; then
  echo ".env.local must have mode 0600" >&2
  exit 2
fi

if [ "${1:-}" = "--" ]; then
  shift
fi
if [ "$#" -eq 0 ]; then
  echo "usage: scripts/with-env-local.sh -- <command> [args...]" >&2
  exit 2
fi

set -a
# shellcheck disable=SC1090
. "$env_file"
set +a

exec "$@"
