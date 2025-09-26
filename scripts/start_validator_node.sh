#!/usr/bin/env bash
set -euo pipefail

# Config
CHAIN_PATH="$HOME/mod-net/modsdk/chain"
SCRIPT_PATH="$CHAIN_PATH/scripts"
NODE_PATH="${CHAIN_PATH}/target/release/modnet-node"
KEY_TOOL="${KEY_TOOL:-$SCRIPT_PATH/key_tools.py}"
INSERT_KEYS_SCRIPT="${INSERT_KEYS_SCRIPT:-$SCRIPT_PATH/insert_session_keys.py}"
KEY_DIR="${KEY_DIR:-$HOME/.modnet/keys}"
LOG_DIR="${LOG_DIR:-$HOME/.modnet/logs}"
RPC_HOST="127.0.0.1"

mkdir -p "$LOG_DIR"

generate_keys() {
  uv run "$KEY_TOOL" gen --scheme sr25519 --name "$NAME" < /dev/tty 2>&1
}

# helpers
json_extract_first_block() {
  # read from stdin, print the first {...} block with trailing commas removed
  perl -0777 -ne 'if (/(\{.*?\})/s) { $j=$1; $j =~ s/,\s*([}\]])/$1/g; print $j }'
}

wait_for_rpc() {
  local rpc_url="$1"
  local timeout="${2:-60}"   # seconds
  local start ts
  start=$(date +%s)
  while true; do
    # test JSON-RPC method system_health (works for substrate nodes)
    if curl -s -H 'Content-Type: application/json' --fail "$rpc_url" \
         -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' >/dev/null 2>&1; then
      return 0
    fi
    ts=$(date +%s)
    if (( ts - start >= timeout )); then
      return 1
    fi
    sleep 1
  done
}

if [[ -z "$NODE_PATH" || ! -x "$NODE_PATH" ]]; then
  echo "ERROR: NODE_PATH not set or not executable: $NODE_PATH" >&2
  exit 1
fi

read -rp "Node type (validator, full, archive): " NODE_TYPE
read -rp "Node number: " NODE_NUMBER

NAME="${NODE_TYPE}-${NODE_NUMBER}"
RPC_PORT="993${NODE_NUMBER}"
RPC_URL="http://${RPC_HOST}:${RPC_PORT}"
LOG_FILE="${LOG_DIR}/${NAME}.log"

# generate key (interactive) and capture output in tmp (for logs only)
tmp_out="$(mktemp)"

# generate aura and grandpa
uv run "$KEY_TOOL" gen-all < /dev/tty 2>&1 | tee "$tmp_out"

echo "Running key generation. If prompted, type passphrase in your terminal."
uv run "$KEY_TOOL" gen --scheme sr25519 --name "$NAME" < /dev/tty 2>&1 | tee -a "$tmp_out"

# load exported key variables from source_keys.sh (AURA/GRANDPA, plus filename-derived)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/source_keys.sh"

# Decide node libp2p key: prefer AURA public, else try any filename-derived VALIDATOR_# key, else empty
NODE_PUB_KEY="${PUBLIC_KEY_HEX_AURA:-}"
if [[ -z "$NODE_PUB_KEY" ]]; then
  # try a sensible filename-derived default based on NAME
  SAFE_NAME="$(printf '%s' "$NAME" | tr '[:lower:]' '[:upper:]' | sed -E 's/[^A-Z0-9]+/_/g')"
  CAND_VAR="${SAFE_NAME}_PUBLIC_HEX"
  # indirect expansion
  NODE_PUB_KEY="${!CAND_VAR:-}"
fi
if [[ -z "$NODE_PUB_KEY" ]]; then
  echo "WARN: Could not determine node public key from sourced environment. Node will start without --node-key." >&2
fi

# Build node args (unsafe flags for validator)
NODE_ARGS=(--chain $CHAIN_PATH/modnet-testnet-raw.json --name "$NAME" --listen-addr "/ip4/0.0.0.0/tcp/3033${NODE_NUMBER}" --rpc-cors all --rpc-port "$RPC_PORT" --base-path "$HOME/.modnet/data")
if [[ -n "$NODE_PUB_KEY" ]]; then
  NODE_ARGS+=(--node-key "$NODE_PUB_KEY")
fi

if [[ "$NODE_TYPE" == "validator" ]]; then
  NODE_ARGS+=(--validator --rpc-methods Unsafe --rpc-external --force-authoring)
fi

# start node in background, log to file
echo "Starting node in background (log -> $LOG_FILE)"
# run node; redirect stdout+stderr to log file
"$NODE_PATH" "${NODE_ARGS[@]}" >"$LOG_FILE" 2>&1 &
NODE_PID=$!
echo "Node started with PID $NODE_PID. Waiting for RPC at $RPC_URL ..."

# ensure we clean up node on exit if script dies
cleanup() {
  echo "Shutting down node (PID $NODE_PID) ..."
  kill "$NODE_PID" 2>/dev/null || true
  wait "$NODE_PID" 2>/dev/null || true
}
trap cleanup EXIT

# wait for RPC
if ! wait_for_rpc "$RPC_URL" 60; then
  echo "ERROR: node RPC did not become available at $RPC_URL in time. See $LOG_FILE" >&2
  exit 1
fi
echo "Node RPC responsive at $RPC_URL"

# If validator, insert session keys
if [[ "$NODE_TYPE" == "validator" ]]; then
  # Prefer paths from sourced environment
  AURA_FILE="${KEY_AURA_PATH:-}"
  GRANDPA_FILE="${KEY_GRANDPA_PATH:-}"

  # fallback: locate aura and grandpa files under KEY_DIR matching name heuristics
  if [[ -z "$AURA_FILE" ]]; then
    AURA_FILE="$(ls -1 "${KEY_DIR}"/*"${NAME}"*aura* 2>/dev/null | head -n1 || true)"
  fi
  if [[ -z "$GRANDPA_FILE" ]]; then
    GRANDPA_FILE="$(ls -1 "${KEY_DIR}"/*"${NAME}"*grandpa* 2>/dev/null | head -n1 || true)"
  fi

  # fallback: try any recent aura/grandpa files if specific ones not found
  if [[ -z "$AURA_FILE" ]]; then
    AURA_FILE="$(ls -1 "${KEY_DIR}"/*aura* 2>/dev/null | tail -n1 || true)"
  fi
  if [[ -z "$GRANDPA_FILE" ]]; then
    GRANDPA_FILE="$(ls -1 "${KEY_DIR}"/*grandpa* 2>/dev/null | tail -n1 || true)"
  fi

  # if still missing, ask user to input paths
  if [[ -z "$AURA_FILE" ]]; then
    read -rp "Could not find an AURA key file automatically. Enter aura file path: " AURA_FILE
  fi
  if [[ -z "$GRANDPA_FILE" ]]; then
    read -rp "Could not find a GRANDPA key file automatically. Enter grandpa file path: " GRANDPA_FILE
  fi

  echo "Inserting session keys. You will be prompted for the key password(s)."
  # Run insert script interactively, reading prompt from your terminal so you can type passphrase
  python3 "$INSERT_KEYS_SCRIPT" --rpc "$RPC_URL" --aura-file "$AURA_FILE" --grandpa-file "$GRANDPA_FILE" --prompt < /dev/tty

  echo "Session keys inserted. Now shutting node down and restarting in safe mode."

  # shutdown node
  cleanup
  # remove trap for normal flow to avoid double-kill
  trap - EXIT

  # small pause to let ports free
  sleep 2

  # restart node in safe mode: remove rpc-external and unsafe rpc-methods and force-authoring if present
  SAFE_ARGS=()
  for a in "${NODE_ARGS[@]}"; do
    # skip flags we want removed
    if [[ "$a" == "--rpc-external" || "$a" == "--rpc-methods" || "$a" == "Unsafe" || "$a" == "--force-authoring" ]]; then
      # skip these tokens
      continue
    fi
    SAFE_ARGS+=("$a")
  done

  echo "Starting node in safe mode (no rpc-external, no Unsafe RPC methods). Log -> $LOG_FILE"
  "$NODE_PATH" "${SAFE_ARGS[@]}" >"$LOG_FILE" 2>&1 &
  NODE_PID=$!
  echo "Safe node started with PID $NODE_PID (log: $LOG_FILE)."
  # re-arm cleanup for this run
  trap cleanup EXIT
fi

echo "Done. Node running. Monitor logs: tail -f $LOG_FILE"
