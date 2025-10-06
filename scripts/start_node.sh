#! /usr/bin/env bash
set -e

$MODNET_NODE_PATH \
  --chain "$MODNET_SPEC" \
  --base-path "$MODNET_CHAIN_DIR/data"