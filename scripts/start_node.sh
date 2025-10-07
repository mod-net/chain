#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHAIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

NODE_BIN="${MODNET_NODE_PATH:-$CHAIN_ROOT/target/release/modnet-node}"
CHAIN_SPEC="${MODNET_SPEC:-$CHAIN_ROOT/specs/modnet-testnet-raw.json}"
CHAIN_DIR="${MODNET_CHAIN_DIR:-$HOME/.modnet}"

if [[ ! -x "$NODE_BIN" ]]; then
  echo "ERROR: node binary not found or not executable at: $NODE_BIN" >&2
  exit 1
fi

exec "$NODE_BIN" \
  --chain "$CHAIN_SPEC" \
  --base-path "$CHAIN_DIR/data"