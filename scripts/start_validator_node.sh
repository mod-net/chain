#!/bin/bash
set -euo pipefail

# Establish canonical paths and load root .env for defaults
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHAIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$CHAIN_ROOT/.." && pwd)"
ENV_FILE="$REPO_ROOT/.env"
if [[ -f "$ENV_FILE" ]]; then
  set -o allexport
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +o allexport
fi

# Canonical defaults matching root .env
MODNET_CHAIN_DIR="${MODNET_CHAIN_DIR:-$HOME/.modnet}"
MODNET_CHAIN_NAME="${MODNET_CHAIN_NAME:-modnet}"
MODNET_CHAIN_PATH="${MODNET_CHAIN_PATH:-$CHAIN_ROOT}"
if [[ "$MODNET_CHAIN_PATH" != /* ]]; then
  MODNET_CHAIN_PATH="$REPO_ROOT/$MODNET_CHAIN_PATH"
fi
MODNET_SCRIPT_PATH="${MODNET_SCRIPT_PATH:-$MODNET_CHAIN_PATH/scripts}"
if [[ "$MODNET_SCRIPT_PATH" != /* ]]; then
  MODNET_SCRIPT_PATH="$REPO_ROOT/$MODNET_SCRIPT_PATH"
fi
MODNET_NODE_PATH="${MODNET_NODE_PATH:-$MODNET_CHAIN_PATH/target/release/modnet-node}"
MODNET_SPEC="${MODNET_SPEC:-$MODNET_CHAIN_PATH/specs/modnet-testnet-raw.json}"
MODNET_LOG_DIR="${MODNET_LOG_DIR:-$MODNET_CHAIN_DIR/logs}"
MODNET_RPC_HOST="${MODNET_RPC_HOST:-127.0.0.1}"
MODNET_KEY_DIR="${MODNET_KEY_DIR:-$MODNET_CHAIN_DIR/keys}"
MODNET_KEYS_SCRIPT="${MODNET_KEYS_SCRIPT:-$MODNET_SCRIPT_PATH/key_tools.py}"
MODNET_KEYS_INSERT_SCRIPT="${MODNET_KEYS_INSERT_SCRIPT:-$MODNET_SCRIPT_PATH/insert_session_keys.py}"

LOG_DIR="$MODNET_LOG_DIR"
CHAIN_SPEC="$MODNET_SPEC"
RPC_HOST="$MODNET_RPC_HOST"
BASE_PATH="$MODNET_CHAIN_DIR/data"

mkdir -p "$LOG_DIR" "$BASE_PATH"
mkdir -p "$MODNET_KEY_DIR"


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
  local bn_file="$MODNET_CHAIN_PATH/specs/boot_nodes.json"
  local added=0
  if [[ -f "$bn_file" ]]; then
    if command -v jq >/dev/null 2>&1; then
      # If value is a string, wrap as array; if array, iterate
      mapfile -t addrs < <(jq -r --arg k "$NAME" '.[ $k ] | if type=="string" then [.] elif type =="array" then . else [] end | .[]' "$bn_file" 2>/dev/null || true)
      for a in "${addrs[@]}"; do
        if [[ -n "$a" ]]; then
          echo "Using bootnode for $NAME from specs: $a"
          NODE_ARGS+=(--bootnodes "$a")
          added=1
        fi
      done
    else
      python3 - "$bn_file" "$NAME" 2>/dev/null <<'PY' | while IFS= read -r a; do
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

resolve_libp2p_key() {
  KEY_LIBP2P=""
  local value="${MODNET_KEY_LIBP2P:-}"
  if [[ -z "$value" ]]; then
    return 1
  fi

  if [[ -f "$value" ]]; then
    KEY_LIBP2P="$(tr -d '\n\r\t ' < "$value" || true)"
  else
    local hex="$value"
    if [[ "$hex" == 0x* || "$hex" == 0X* ]]; then
      hex="${hex:2}"
    fi
    if [[ "$hex" =~ ^[0-9a-fA-F]{64}$ ]]; then
      KEY_LIBP2P="$hex"
    else
      echo "ERROR: MODNET_KEY_LIBP2P must be a path to a node key file or a 32-byte hex string." >&2
      return 2
    fi
  fi

  if [[ -z "$KEY_LIBP2P" ]]; then
    echo "ERROR: Resolved libp2p node key is empty." >&2
    return 2
  fi

  return 0
}

# Obtain or generate a libp2p node key (ed25519 secret) for --node-key
get_or_generate_node_key() {
  # Respect environment override if provided
  if [[ -n "${MODNET_KEY_LIBP2P:-}" ]]; then
    if ! resolve_libp2p_key; then
      echo "ERROR: MODNET_KEY_LIBP2P is invalid. Please check the value." >&2
      exit 1
    fi
    echo "Using MODNET_KEY_LIBP2P from environment"
{{ ... }}
    return 0
  fi
  read -p "Generate a persistent libp2p node key for '$NAME'? (y/n): " gen_nodekey
  if [[ "$gen_nodekey" != "y" ]]; then
    return 1
  fi
  if ! command -v subkey >/dev/null 2>&1; then
    echo "ERROR: 'subkey' is required to generate the node key. Install Substrate subkey tool." >&2
    return 1
  fi
  echo "Generating libp2p node key with 'subkey generate-node-key'..."
  local key_hex
  if ! key_hex="$(subkey generate-node-key 2>/dev/null)"; then
    echo "ERROR: Failed to generate node key via subkey" >&2
    return 1
  fi
  # subkey prints the key hex; trim whitespace just in case
  key_hex="$(printf '%s' "$key_hex" | tr -d '[:space:]')"
  # Persist for reuse
  local key_dir="$MODNET_KEY_DIR"
  mkdir -p "$key_dir"
  local key_file="$key_dir/nodekey-${NAME}.hex"
  printf '%s\n' "$key_hex" >"$key_file"
  chmod 600 "$key_file" || true
  export MODNET_KEY_LIBP2P="$key_file"
  echo "Saved libp2p node key to $key_file and will pass via --node-key."
  return 0
}
if [[ -z "$MODNET_NODE_PATH" || ! -x "$MODNET_NODE_PATH" ]]; then
  echo "ERROR: MODNET_NODE_PATH not set or not executable: $MODNET_NODE_PATH" >&2
  exit 1
fi

read -rp "Node type (validator, full, archive): " NODE_TYPE
read -rp "Node number: " NODE_NUMBER

bash "$MODNET_SCRIPT_PATH/setup_ngrok.sh" "$NODE_TYPE" "$NODE_NUMBER"

NAME="${NODE_TYPE}-${NODE_NUMBER}"
# Ensure base-path subdirectories exist so the node can write its network key if needed
mkdir -p "${MODNET_CHAIN_DIR:-$HOME/.modnet}/data/chains/${MODNET_CHAIN_NAME:-modnet}-testnet/network"

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
  bash "$MODNET_SCRIPT_PATH/generate_keys.sh" --name "$NAME"
fi

# Optionally use PM2 to run the node
read -p "Run node under PM2? (y/n): " use_pm2

# load exported key variables from source_keys.sh (AURA/GRANDPA, plus filename-derived)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Ensure canonical env is set for the loader
export TARGET_DIR="$MODNET_KEY_DIR"
export MODNET_KEYS_SCRIPT="$MODNET_KEYS_SCRIPT"
# Source keys helper
source "$SCRIPT_DIR/source_keys.sh"

if ! resolve_libp2p_key; then
  echo "INFO: MODNET_KEY_LIBP2P not set; will continue without --node-key unless generated." >&2
fi

# Give a chance to generate/pick a libp2p node key to avoid NetworkKeyNotFound
get_or_generate_node_key || true

# Re-resolve in case generation provided a key file or hex
resolve_libp2p_key || true

# Map MODNET key paths to expected variables for key insertion flow
if [[ -n "${MODNET_KEY_AURA:-}" ]]; then
  export KEY_AURA_PATH="$MODNET_KEY_AURA"
fi
if [[ -n "${MODNET_KEY_GRANDPA:-}" ]]; then
  export KEY_GRANDPA_PATH="$MODNET_KEY_GRANDPA"
fi

# Build node args (unsafe flags for validator)
NODE_ARGS=(--chain "$CHAIN_SPEC" --name "$NAME" --listen-addr "/ip4/0.0.0.0/tcp/${P2P_PORT}" --rpc-cors all --rpc-port "$RPC_PORT" --base-path "$MODNET_CHAIN_DIR/data" --prometheus-external --prometheus-port "$PROM_PORT")
if [[ -n "$KEY_LIBP2P" ]]; then
  NODE_ARGS+=(--node-key "$KEY_LIBP2P")
else
  echo "INFO: Starting without an explicit libp2p --node-key; the node will generate a temporary key." >&2
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
  pm2 start "$MODNET_NODE_PATH" --name "$NAME" --output "$LOG_FILE" --error "$LOG_FILE" -- "${NODE_ARGS[@]}"
  echo "Node started under PM2 as '$NAME'. Waiting for RPC at $RPC_URL ..."
else
  echo "Starting node in background (log -> $LOG_FILE)"
  "$MODNET_NODE_PATH" "${NODE_ARGS[@]}" >"$LOG_FILE" 2>&1 &
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
    python3 "${MODNET_KEYS_INSERT_SCRIPT:-$MODNET_SCRIPT_PATH/insert_session_keys.py}" --rpc "$RPC_URL" --aura-file "$AURA_FILE" --grandpa-file "$GRANDPA_FILE" --prompt < /dev/tty

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
      pm2 start "$MODNET_NODE_PATH" --name "$NAME" --output "$LOG_FILE" --error "$LOG_FILE" -- "${SAFE_ARGS[@]}"
    else
      echo "Session keys inserted. Now shutting node down and restarting in safe mode."
      # shutdown node (non-PM2)
      cleanup
      # remove trap for normal flow to avoid double-kill
      trap - INT TERM
      # small pause to let ports free
      sleep 2
      echo "Starting node in safe mode (no rpc-external, no Unsafe RPC methods). Log -> $LOG_FILE"
      "$MODNET_NODE_PATH" "${SAFE_ARGS[@]}" >"$LOG_FILE" 2>&1 &
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


