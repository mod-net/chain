#!/bin/bash
set -euo pipefail

# Config
CHAIN_PATH="$HOME/mod-net/modsdk/chain"
SCRIPT_PATH="$CHAIN_PATH/scripts"
NODE_PATH="${CHAIN_PATH}/target/release/modnet-node"
INSERT_KEYS_SCRIPT="${INSERT_KEYS_SCRIPT:-$SCRIPT_PATH/insert_session_keys.py}"
LOG_DIR="${LOG_DIR:-$HOME/.modnet/logs}"
RPC_HOST="127.0.0.1"

mkdir -p "$LOG_DIR"


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

# Try to load a bootnode multiaddr from specs/boot_nodes.json for this node NAME
load_bootnode_from_specs() {
  local bn_file="$CHAIN_PATH/specs/boot_nodes.json"
  local added=0
  if [[ -f "$bn_file" ]]; then
    if command -v jq >/dev/null 2>&1; then
      # If value is a string, wrap as array; if array, iterate
      mapfile -t addrs < <(jq -r --arg k "$NAME" '.[ $k ] | if type=="string" then [.] elif type=="array" then . else [] end | .[]' "$bn_file" 2>/dev/null || true)
      for a in "${addrs[@]}"; do
        if [[ -n "$a" ]]; then
          echo "Using bootnode for $NAME from specs: $a"
          NODE_ARGS+=(--bootnodes "$a")
          added=1
        fi
      done
    else
      python3 - "$bn_file" "$NAME" <<'PY' 2>/dev/null
import sys, json
path, key = sys.argv[1], sys.argv[2]
try:
    with open(path) as f:
        data = json.load(f)
    v = data.get(key)
    addrs = []
    if isinstance(v, str):
        addrs = [v]
    elif isinstance(v, list):
        addrs = [x for x in v if isinstance(x, str)]
    for a in addrs:
        print(a)
except Exception:
    pass
PY
      | while IFS= read -r a; do
          if [[ -n "$a" ]]; then
            echo "Using bootnode for $NAME from specs: $a"
            NODE_ARGS+=(--bootnodes "$a")
            added=1
          fi
        done
    fi
  fi
  if (( added > 0 )); then
    return 0
  fi
  return 1
}

if [[ -z "$NODE_PATH" || ! -x "$NODE_PATH" ]]; then
  echo "ERROR: NODE_PATH not set or not executable: $NODE_PATH" >&2
  exit 1
fi

read -rp "Node type (validator, full, archive): " NODE_TYPE
read -rp "Node number: " NODE_NUMBER

bash "$SCRIPT_PATH/setup_ngrok.sh" "$NODE_TYPE" "$NODE_NUMBER"

NAME="${NODE_TYPE}-${NODE_NUMBER}"
RPC_PORT=""
P2P_PORT=""
PROM_PORT=""

# Auto-derive ports per node type to avoid clashes across roles
case "$NODE_TYPE" in
  validator)
    P2P_PORT="3033${NODE_NUMBER}"
    RPC_PORT="993${NODE_NUMBER}"
    PROM_PORT="961${NODE_NUMBER}"
    ;;
  full)
    P2P_PORT="3043${NODE_NUMBER}"
    RPC_PORT="994${NODE_NUMBER}"
    PROM_PORT="962${NODE_NUMBER}"
    ;;
  archive)
    P2P_PORT="3053${NODE_NUMBER}"
    RPC_PORT="995${NODE_NUMBER}"
    PROM_PORT="963${NODE_NUMBER}"
    ;;
  *)
    # default fallbacks
    P2P_PORT="3033${NODE_NUMBER}"
    RPC_PORT="993${NODE_NUMBER}"
    PROM_PORT="961${NODE_NUMBER}"
    ;;
esac

RPC_URL="http://${RPC_HOST}:${RPC_PORT}"
LOG_FILE="${LOG_DIR}/${NAME}.log"

# Optionally generate keys before sourcing
read -p "Generate keys now for '$NAME'? (y/n): " gen_now
if [[ "$gen_now" == "y" ]]; then
  bash "$SCRIPT_PATH/generate_keys.sh" --name "$NAME"
fi

# Optionally use PM2 to run the node
read -p "Run node under PM2? (y/n): " use_pm2

# load exported key variables from source_keys.sh (AURA/GRANDPA, plus filename-derived)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Ensure source_keys.sh uses the local key_tools.py by default
export KEY_TOOL_SCRIPT="$SCRIPT_DIR/key_tools.py"
source "$SCRIPT_DIR/source_keys.sh"

# Decide node libp2p key only if explicitly provided by environment
NODE_LIBP2P_KEY="${NODE_LIBP2P_KEY:-}"
if [[ -z "$NODE_LIBP2P_KEY" ]]; then
  echo "INFO: NODE_LIBP2P_KEY not set; starting without --node-key."
fi


# Build node args (unsafe flags for validator)
NODE_ARGS=(--chain $CHAIN_PATH/specs/modnet-testnet-raw.json --name "$NAME" --listen-addr "/ip4/0.0.0.0/tcp/${P2P_PORT}" --rpc-cors all --rpc-port "$RPC_PORT" --base-path "$HOME/.modnet/data" --prometheus-external --prometheus-port "$PROM_PORT")
if [[ -n "$NODE_LIBP2P_KEY" ]]; then
  NODE_ARGS+=(--node-key "$NODE_LIBP2P_KEY")
fi

# Telemetry selection
read -p "Use public Polkadot telemetry? (y/n): " use_public_telemetry
if [[ "$use_public_telemetry" == "y" ]]; then
  NODE_ARGS+=(--telemetry-url "wss://telemetry.polkadot.io/submit 0")
else
  NODE_ARGS+=(--no-telemetry)
fi

if [[ "$NODE_TYPE" == "validator" ]]; then
  # Ask if this node should run with --validator flag
  read -p "Run with --validator role? (y/n): " use_validator_flag
  if [[ "$use_validator_flag" == "y" ]]; then
    NODE_ARGS+=(--validator)
  fi
  read -p "Run node in unsafe mode? (y/n): " unsafe_mode
  if [[ "$unsafe_mode" == "y" ]]; then
    NODE_ARGS+=(--rpc-methods Unsafe --rpc-external)
  else
    NODE_ARGS+=(--rpc-methods Safe)
  fi
fi

# Optional bootnode
if ! load_bootnode_from_specs; then
  read -p "Add a bootnode multiaddr? (y/n): " add_bootnode
  if [[ "$add_bootnode" == "y" ]]; then
    read -rp "Enter bootnode multiaddr (e.g., /ip4/1.2.3.4/tcp/30333/p2p/12D3...): " BOOTNODE_ADDR
    if [[ -n "${BOOTNODE_ADDR// }" ]]; then
      NODE_ARGS+=(--bootnodes "$BOOTNODE_ADDR")
    fi
  fi
fi

# Optional force authoring (useful for single-node local dev)
read -p "Force authoring (single-node dev)? (y/n): " force_auth
if [[ "$force_auth" == "y" ]]; then
  NODE_ARGS+=(--force-authoring)
fi

# start node
if [[ "$use_pm2" == "y" ]]; then
  echo "Starting node with PM2 (log -> $LOG_FILE)"
  # Start under PM2; use -- to pass args to the binary. Use same file for both out/err.
  pm2 start "$NODE_PATH" --name "$NAME" --output "$LOG_FILE" --error "$LOG_FILE" -- "${NODE_ARGS[@]}"
  echo "Node started under PM2 as '$NAME'. Waiting for RPC at $RPC_URL ..."
else
  echo "Starting node in background (log -> $LOG_FILE)"
  "$NODE_PATH" "${NODE_ARGS[@]}" >"$LOG_FILE" 2>&1 &
  NODE_PID=$!
  echo "Node started with PID $NODE_PID. Waiting for RPC at $RPC_URL ..."
fi

if [[ "${use_pm2:-n}" != "y" ]]; then
  # ensure we clean up node on interrupt/termination (non-PM2 only)
  cleanup() {
    echo "Shutting down node (PID $NODE_PID) ..."
    kill "$NODE_PID" 2>/dev/null || true
    wait "$NODE_PID" 2>/dev/null || true
  }
  trap cleanup INT TERM
fi

# wait for RPC
if ! wait_for_rpc "$RPC_URL" 60; then
  echo "ERROR: node RPC did not become available at $RPC_URL in time. See $LOG_FILE" >&2
  exit 1
fi
echo "Node RPC responsive at $RPC_URL"

# If validator and chosen unsafe mode, insert session keys then restart in safe mode
if [[ "$NODE_TYPE" == "validator" ]]; then
  if [[ "${unsafe_mode:-n}" == "y" ]]; then
    # Use paths from sourced environment only
    AURA_FILE="${KEY_AURA_PATH:-}"
    GRANDPA_FILE="${KEY_GRANDPA_PATH:-}"
    if [[ -z "$AURA_FILE" || -z "$GRANDPA_FILE" ]]; then
      echo "ERROR: KEY_AURA_PATH or KEY_GRANDPA_PATH not set. Run scripts/source_keys.sh first to load keys." >&2
      exit 1
    fi

    echo "Inserting session keys. You will be prompted for the key password(s)."
    # Run insert script interactively, reading prompt from your terminal so you can type passphrase
    python3 "$INSERT_KEYS_SCRIPT" --rpc "$RPC_URL" --aura-file "$AURA_FILE" --grandpa-file "$GRANDPA_FILE" --prompt < /dev/tty

    echo "Session keys inserted. Now shutting node down and restarting in safe mode."

    # restart node in safe mode: remove rpc-external and unsafe rpc-methods and force-authoring if present
    SAFE_ARGS=()
    for a in "${NODE_ARGS[@]}"; do
      # skip flags we want removed
      if [[ "$a" == "--rpc-external" || "$a" == "--rpc-methods" || "$a" == "Unsafe" || "$a" == "--force-authoring" ]]; then
        continue
      fi
      SAFE_ARGS+=("$a")
    done

    if [[ "$use_pm2" == "y" ]]; then
      echo "Restarting PM2 process '$NAME' in safe mode (no rpc-external/Unsafe)."
      pm2 delete "$NAME" || true
      pm2 start "$NODE_PATH" --name "$NAME" --output "$LOG_FILE" --error "$LOG_FILE" -- "${SAFE_ARGS[@]}"
    else
      echo "Session keys inserted. Now shutting node down and restarting in safe mode."
      # shutdown node (non-PM2)
      cleanup
      # remove trap for normal flow to avoid double-kill
      trap - INT TERM
      # small pause to let ports free
      sleep 2
      echo "Starting node in safe mode (no rpc-external, no Unsafe RPC methods). Log -> $LOG_FILE"
      "$NODE_PATH" "${SAFE_ARGS[@]}" >"$LOG_FILE" 2>&1 &
      NODE_PID=$!
      echo "Safe node started with PID $NODE_PID (log: $LOG_FILE)."
      # re-arm cleanup for this run
      trap cleanup INT TERM
    fi
  else
    echo "Safe mode chosen: skipping session key insertion and restart. Node continues running in safe mode." 
  fi
fi

if [[ "$use_pm2" == "y" ]]; then
  echo "Done. Node running under PM2 as '$NAME'. View logs with: pm2 logs $NAME"
else
  echo "Done. Node running. Monitor logs: tail -f $LOG_FILE"
fi
