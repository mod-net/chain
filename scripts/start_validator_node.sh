#!/usr/bin/env bash
set -e

NODE_PATH="$(pwd)/target/release/modnet-node"

# Start validator with HTTP RPC on 9933 and WebSocket on 9944.
# Expose RPC/WS externally with permissive CORS for development/testing.
"$NODE_PATH" \
  --chain specs/modnet-testnet-raw.json \
  --validator \
  --name BootNode-01 \
  --node-key 40ffa204f07664248b1d10d5a57a28877206fb82ac356f9273824dae81375e81 \
  --listen-addr /ip4/0.0.0.0/tcp/30333 \
  --rpc-cors all \
  --rpc-port 9944 \
  --rpc-methods Safe \
  --force-authoring \
  --base-path ~/.modnet/data