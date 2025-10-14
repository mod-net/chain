#!/usr/bin/env bash
set -euo pipefail

# Simple launcher for a non-validator full node.
# You can override defaults using environment variables:
#   NODE_NAME       - Node name (default: FullNode-01)
#   LISTEN_PORT     - P2P listen TCP port (default: 30335)
#   RPC_PORT        - HTTP RPC port (default: 9934)
#   BASE_PATH       - Base path for chain data (default: ~/.modnet/data-full-1)
#   BOOTNODE        - Optional libp2p multiaddr for bootnode (e.g., /ip4/1.2.3.4/tcp/30333/p2p/<peerId>)
#   CHAIN_SPEC      - Path to chain spec (default: specs/modnet-testnet-raw.json)

NODE_PATH="$(pwd)/target/release/modnet-node"
NODE_NAME=${NODE_NAME:-FullNode-01}
LISTEN_PORT=${LISTEN_PORT:-30335}
RPC_PORT=${RPC_PORT:-9934}
BASE_PATH=${BASE_PATH:-$HOME/.modnet/data-full-1}
CHAIN_SPEC=${CHAIN_SPEC:-specs/modnet-testnet-raw.json}

CMD=(
  "$NODE_PATH"
  --chain "$CHAIN_SPEC"
  --name "$NODE_NAME"
  --listen-addr "/ip4/0.0.0.0/tcp/${LISTEN_PORT}"
  --rpc-cors all
  --rpc-port "$RPC_PORT"
  --rpc-methods Safe
  --base-path "$BASE_PATH"
)

if [[ -n "${BOOTNODE:-}" ]]; then
  CMD+=(--bootnodes "$BOOTNODE")
fi

exec "${CMD[@]}"
