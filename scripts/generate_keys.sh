#!/usr/bin/env bash
set -euo pipefail

# This helper generates validator and session keys using scripts/key_tools.py
# It is fully interactive (reads passphrases from your TTY) and optional per prompt.
# Usage:
#   scripts/generate_keys.sh [--name NODE_NAME]
# If --name is omitted, you will be prompted for it when needed.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEY_TOOL_SCRIPT="${KEY_TOOL_SCRIPT:-$SCRIPT_DIR/key_tools.py}"
sudo apt-get install clang
cargo install subkey 

if [[ ! -f "$KEY_TOOL_SCRIPT" ]]; then
  echo "ERROR: key tool not found at: $KEY_TOOL_SCRIPT" >&2
  exit 1
fi

NODE_NAME=""
if [[ ${1:-} == "--name" ]]; then
  NODE_NAME="${2:-}"
fi

read -p "Generate AURA and GRANDPA session keys now? (y/n): " gen_all
if [[ "$gen_all" == "y" ]]; then
  echo "Generating AURA/GRANDPA session keys..."
  uv run "$KEY_TOOL_SCRIPT" gen-all < /dev/tty 2>&1
  echo "Done generating session keys."
fi

if [[ -z "$NODE_NAME" ]]; then
  read -rp "Node name for validator key (e.g., validator-1): " NODE_NAME
fi

read -p "Generate validator key for '$NODE_NAME'? (y/n): " gen_validator
if [[ "$gen_validator" == "y" ]]; then
  echo "Generating validator key for '$NODE_NAME' (you may be prompted for a passphrase)..."
  uv run "$KEY_TOOL_SCRIPT" gen --scheme sr25519 --name "$NODE_NAME" < /dev/tty 2>&1
  echo "Done generating validator key."
fi

echo "Key generation flow completed."
