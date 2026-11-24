#!/usr/bin/env bash
set -u

export FORGE_PORT="${FORGE_PORT:-8080}"
export FORGE_ENV="${FORGE_ENV:-development}"
export FORGE_DATABASE_URL="${FORGE_DATABASE_URL:-sqlite://forge.db?mode=rwc}"
export FORGE_DEV_API_TOKEN="${FORGE_DEV_API_TOKEN:-forge-dev-local}"
export FORGE_API_URL="${FORGE_API_URL:-http://localhost:${FORGE_PORT}}"
export FORGE_API_TOKEN="${FORGE_API_TOKEN:-${FORGE_DEV_API_TOKEN}}"

cargo run -p forge-server &
server_pid=$!
pnpm --filter @forge/web dev &
web_pid=$!

cleanup() {
  kill "$server_pid" "$web_pid" 2>/dev/null || true
  wait "$server_pid" "$web_pid" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

wait -n "$server_pid" "$web_pid"
status=$?
exit "$status"
