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

to_bool() {
  case "${1:-}" in
    y|Y|yes|YES|true|TRUE|1)
      echo "y"
      ;;
    n|N|no|NO|false|FALSE|0)
      echo "n"
      ;;
    *)
      echo ""
      ;;
  esac
}

usage() {
  cat <<'EOF'
Usage: start_validator_node.sh [OPTIONS]

Required options (when running non-interactively):
  --type <validator|full|archive>   Node role. Can also be set via MODNET_NODE_TYPE.
  --number <index>                  Node number (e.g. 1). Can also be set via MODNET_NODE_NUMBER.

Common automation flags:
  --pm2 <y|n>                       Run under PM2 (default: prompt or MODNET_RUN_PM2).
  --unsafe <y|n>                    Launch in unsafe mode to insert session keys.
  --force-authoring <y|n>           Enable --force-authoring.
  --telemetry <public|none>         Telemetry mode (public Polkadot or disabled).
  --generate-keys <y|n>             Run generate_keys.sh before starting.
  --bootnode <multiaddr>            Add explicit bootnode (repeatable).
  --ngrok <skip|configure|start>    Control ngrok integration (default: ask).
  --libp2p <hex|path>               Provide libp2p node key hex or file path.
  --auto-generate-libp2p <y|n>      Auto-create libp2p key if none provided.

Any option can alternatively be provided via the corresponding MODNET_* environment variable.
EOF
}

CLI_NODE_TYPE=""
CLI_NODE_NUMBER=""
CLI_RUN_PM2=""
CLI_UNSAFE_MODE=""
CLI_FORCE_AUTHORING=""
CLI_GENERATE_KEYS=""
CLI_TELEMETRY=""
CLI_NGROK_MODE=""
CLI_LIBP2P=""
CLI_AUTO_LIBP2P=""
declare -a CLI_BOOTNODES=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --type)
      [[ $# -ge 2 ]] || { echo "ERROR: --type requires a value" >&2; usage; exit 1; }
      CLI_NODE_TYPE="$2"
      shift 2
      ;;
    --number)
      [[ $# -ge 2 ]] || { echo "ERROR: --number requires a value" >&2; usage; exit 1; }
      CLI_NODE_NUMBER="$2"
      shift 2
      ;;
    --pm2)
      [[ $# -ge 2 ]] || { echo "ERROR: --pm2 requires a value" >&2; usage; exit 1; }
      CLI_RUN_PM2="$(to_bool "$2")"
      if [[ -z "$CLI_RUN_PM2" ]]; then
        echo "ERROR: --pm2 expects y/n" >&2
        exit 1
      fi
      shift 2
      ;;
    --unsafe)
      [[ $# -ge 2 ]] || { echo "ERROR: --unsafe requires a value" >&2; usage; exit 1; }
      CLI_UNSAFE_MODE="$(to_bool "$2")"
      if [[ -z "$CLI_UNSAFE_MODE" ]]; then
        echo "ERROR: --unsafe expects y/n" >&2
        exit 1
      fi
      shift 2
      ;;
    --force-authoring)
      [[ $# -ge 2 ]] || { echo "ERROR: --force-authoring requires a value" >&2; usage; exit 1; }
      CLI_FORCE_AUTHORING="$(to_bool "$2")"
      if [[ -z "$CLI_FORCE_AUTHORING" ]]; then
        echo "ERROR: --force-authoring expects y/n" >&2
        exit 1
      fi
      shift 2
      ;;
    --generate-keys)
      [[ $# -ge 2 ]] || { echo "ERROR: --generate-keys requires a value" >&2; usage; exit 1; }
      CLI_GENERATE_KEYS="$(to_bool "$2")"
      if [[ -z "$CLI_GENERATE_KEYS" ]]; then
        echo "ERROR: --generate-keys expects y/n" >&2
        exit 1
      fi
      shift 2
      ;;
    --telemetry)
      [[ $# -ge 2 ]] || { echo "ERROR: --telemetry requires a value" >&2; usage; exit 1; }
      case "$2" in
        public|none)
          CLI_TELEMETRY="$2"
          ;;
        *)
          echo "ERROR: --telemetry expects 'public' or 'none'" >&2
          exit 1
          ;;
      esac
      shift 2
      ;;
    --bootnode)
      [[ $# -ge 2 ]] || { echo "ERROR: --bootnode requires a value" >&2; usage; exit 1; }
      CLI_BOOTNODES+=("$2")
      shift 2
      ;;
    --ngrok)
      [[ $# -ge 2 ]] || { echo "ERROR: --ngrok requires a value" >&2; usage; exit 1; }
      case "$2" in
        skip|configure|start)
          CLI_NGROK_MODE="$2"
          ;;
        *)
          echo "ERROR: --ngrok expects skip|configure|start" >&2
          exit 1
          ;;
      esac
      shift 2
      ;;
    --libp2p)
      [[ $# -ge 2 ]] || { echo "ERROR: --libp2p requires a value" >&2; usage; exit 1; }
      CLI_LIBP2P="$2"
      shift 2
      ;;
    --auto-generate-libp2p)
      [[ $# -ge 2 ]] || { echo "ERROR: --auto-generate-libp2p requires a value" >&2; usage; exit 1; }
      CLI_AUTO_LIBP2P="$(to_bool "$2")"
      if [[ -z "$CLI_AUTO_LIBP2P" ]]; then
        echo "ERROR: --auto-generate-libp2p expects y/n" >&2
        exit 1
      fi
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

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
  if [[ -n "${CLI_LIBP2P:-}" ]]; then
    MODNET_KEY_LIBP2P="$CLI_LIBP2P"
  fi

  if [[ -n "${MODNET_KEY_LIBP2P:-}" ]]; then
    if ! resolve_libp2p_key; then
      echo "ERROR: Provided libp2p key is invalid: $MODNET_KEY_LIBP2P" >&2
      exit 1
    fi
    echo "Using libp2p node key from configuration"
    return 0
  fi

  local should_generate="$(to_bool "${CLI_AUTO_LIBP2P:-${MODNET_AUTO_GENERATE_LIBP2P:-n}}")"
  if [[ "$should_generate" != "y" ]]; then
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

NODE_TYPE="${CLI_NODE_TYPE:-${MODNET_NODE_TYPE:-}}"
NODE_NUMBER="${CLI_NODE_NUMBER:-${MODNET_NODE_NUMBER:-}}"

if [[ -z "$NODE_TYPE" || -z "$NODE_NUMBER" ]]; then
  echo "ERROR: Node type and number must be provided via CLI (--type/--number) or environment." >&2
  usage
  exit 1
fi

NODE_TYPE="${NODE_TYPE,,}"

case "$NODE_TYPE" in
  validator|full|archive)
    ;;
  *)
    echo "ERROR: Invalid node type '$NODE_TYPE'." >&2
    usage
    exit 1
    ;;
esac

NGROK_MODE="${CLI_NGROK_MODE:-${MODNET_NGROK_MODE:-configure}}"

case "$NGROK_MODE" in
  configure)
    bash "$MODNET_SCRIPT_PATH/setup_ngrok.sh" "$NODE_TYPE" "$NODE_NUMBER"
    ;;
  start)
    bash "$MODNET_SCRIPT_PATH/setup_ngrok.sh" "$NODE_TYPE" "$NODE_NUMBER" --auto
    ;;
  skip)
    ;;
  *)
    echo "ERROR: Invalid ngrok mode '$NGROK_MODE'." >&2
    exit 1
    ;;
esac

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
GENERATE_KEYS="$(to_bool "${CLI_GENERATE_KEYS:-${MODNET_GENERATE_KEYS:-n}}")"
if [[ "$GENERATE_KEYS" == "y" ]]; then
  bash "$MODNET_SCRIPT_PATH/generate_keys.sh" --name "$NAME"
fi

USE_PM2="${CLI_RUN_PM2:-${MODNET_RUN_PM2:-}}"
if [[ -z "$USE_PM2" ]]; then
  USE_PM2="$(to_bool "${MODNET_RUN_PM2:-n}")"
fi

if [[ -z "$USE_PM2" ]]; then
  echo "ERROR: Unable to determine PM2 usage. Set --pm2 y|n or MODNET_RUN_PM2." >&2
  exit 1
fi

# load exported key variables from source_keys.sh (AURA/GRANDPA, plus filename-derived)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export TARGET_DIR="$MODNET_KEY_DIR"
export MODNET_KEYS_SCRIPT="$MODNET_KEYS_SCRIPT"
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
TELEMETRY_MODE="${CLI_TELEMETRY:-${MODNET_TELEMETRY_MODE:-public}}"
case "$TELEMETRY_MODE" in
  public)
    NODE_ARGS+=(--telemetry-url "wss://telemetry.polkadot.io/submit 0")
    ;;
  none)
    NODE_ARGS+=(--no-telemetry)
    ;;
  *)
    echo "ERROR: Unsupported telemetry mode '$TELEMETRY_MODE'" >&2
    exit 1
    ;;
esac

if [[ "$NODE_TYPE" == "validator" ]]; then
  NODE_ARGS+=(--validator)
  UNSAFE_MODE="$(to_bool "${CLI_UNSAFE_MODE:-${MODNET_UNSAFE_MODE:-n}}")"
  if [[ "$UNSAFE_MODE" == "y" ]]; then
    NODE_ARGS+=(--rpc-methods Unsafe --rpc-external)
  else
    NODE_ARGS+=(--rpc-methods Safe)
  fi
else
  UNSAFE_MODE="n"
fi

# Optional bootnode
if ! load_bootnode_from_specs; then
  for b in "${CLI_BOOTNODES[@]}" ${MODNET_BOOTNODE_MULTIADDR:-}; do
    if [[ -n "${b// }" ]]; then
      NODE_ARGS+=(--bootnodes "$b")
    fi
  done
fi

# Optional force authoring (useful for single-node dev)
if [[ "$(to_bool "${CLI_FORCE_AUTHORING:-${MODNET_FORCE_AUTHORING:-n}}")" == "y" ]]; then
  NODE_ARGS+=(--force-authoring)
fi

# start node
if [[ "$USE_PM2" == "y" ]]; then
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

if [[ "$USE_PM2" != "y" ]]; then
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
  if [[ "$UNSAFE_MODE" == "y" ]]; then
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

    if [[ "$USE_PM2" == "y" ]]; then
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

if [[ "$USE_PM2" == "y" ]]; then
  echo "Done. Node running under PM2 as '$NAME'. View logs with: pm2 logs $NAME"
else
  echo "Done. Node running. Monitor logs: tail -f $LOG_FILE"
fi


