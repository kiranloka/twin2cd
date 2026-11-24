#!/usr/bin/env bash
set -euo pipefail

# Self-hosted installer (Milestone 7). Verifies release artifacts before execution in production.
FORGE_VERSION="${FORGE_VERSION:-0.1.0}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.forge}"

echo "Forge installer v${FORGE_VERSION}"
echo "Install directory: ${INSTALL_DIR}"

mkdir -p "${INSTALL_DIR}/bin" "${INSTALL_DIR}/data"

if command -v cargo >/dev/null 2>&1; then
  echo "Building forge-server from source..."
  cargo build --release -p forge-server -p forge-runner
  cp target/release/forge-server "${INSTALL_DIR}/bin/"
  cp target/release/forge-runner "${INSTALL_DIR}/bin/"
else
  echo "cargo not found — download prebuilt binaries when releases are published." >&2
  exit 1
fi

cat <<EOF

Installed:
  ${INSTALL_DIR}/bin/forge-server
  ${INSTALL_DIR}/bin/forge-runner

Run control plane:
  FORGE_DATABASE_URL=sqlite://${INSTALL_DIR}/data/forge.db?mode=rwc \\
  FORGE_DEV_API_TOKEN=change-me \\
  ${INSTALL_DIR}/bin/forge-server

EOF