#!/bin/bash

set -euo pipefail

# Source keys using canonical script path
source "${MODNET_SCRIPT_PATH:-scripts}/source_keys.sh"

# Read signatories from environment (set in .env), with no defaults here
SIGNATORY_1="${MODNET_SIGNATORY_1:-}"
SIGNATORY_2="${MODNET_SIGNATORY_2:-}"
SIGNATORY_3="${MODNET_SIGNATORY_3:-}"

# Validate signatories are provided
if [[ -z "$SIGNATORY_1" || -z "$SIGNATORY_2" || -z "$SIGNATORY_3" ]]; then
  echo "ERROR: MODNET_SIGNATORY_1/2/3 must be set in your environment (.env)." >&2
  exit 1
fi

# Derive chain id from MODNET_CHAIN_NAME
CHAIN_ID="${MODNET_CHAIN_NAME:-modnet}-testnet"

uv run "${MODNET_SCRIPT_PATH:-scripts}/init_network.py" \
    --chain-id "$CHAIN_ID" \
    --aura "${SS58_AURA:-$PUBLIC_KEY_HEX_AURA}" \
    --grandpa "${SS58_GRANDPA:-$PUBLIC_KEY_HEX_GRANDPA}" \
    --signer "$SIGNATORY_1" \
    --signer "$SIGNATORY_2" \
    --signer "$SIGNATORY_3" \
    --threshold 2 \
    ${BOOTNODE_MULTIADDR:+--bootnode ${BOOTNODE_MULTIADDR}}