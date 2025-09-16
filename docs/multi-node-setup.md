# Multi-node Setup

This guide shows how to run a validator and an additional full node for local/testnet setups.

## Prerequisites

- Build the node:
  ```bash
  cargo build -r
  ```
- Node binary will be at `target/release/modnet-node`.
- Chain specs are in `specs/modnet-testnet-raw.json`.

## Start a validator (Node 1)

Use the helper script `scripts/start_validator_node.sh`:

```bash
./scripts/start_validator_node.sh
```

Defaults:
- P2P: 30333
- HTTP RPC: 9933
- WebSocket: 9944 (default, no explicit flags needed)
- Base path: `~/.modnet/data`

Note:
- Validators should NOT expose RPC externally; the script keeps `--rpc-methods Safe` and no `--rpc-external`.

## Obtain the bootnode multiaddr

From Node 1 logs, copy the `/p2p/...` multiaddr, e.g.:

```
/ip4/127.0.0.1/tcp/30333/p2p/<peer_id>
```

If you can’t see it, you can query the peer id with:

```bash
./target/release/modnet-node --version
```

Or use the system logs; typically Substrate prints the libp2p address shortly after startup.

## Start a full node (Node 2)

Use the helper `scripts/start_full_node.sh`. You can override defaults via env vars.

Example:
```bash
BOOTNODE="/ip4/127.0.0.1/tcp/30333/p2p/<peer_id>" \
NODE_NAME=FullNode-02 \
LISTEN_PORT=30334 \
RPC_PORT=9934 \
BASE_PATH=$HOME/.modnet/data-full-2 \
./scripts/start_full_node.sh
```

Defaults (can be overridden):
- `NODE_NAME=FullNode-01`
- `LISTEN_PORT=30335`
- `RPC_PORT=9934`
- `BASE_PATH=~/.modnet/data-full-1`
- `CHAIN_SPEC=specs/modnet-testnet-raw.json`

## Insert session keys for additional validators (optional)

If you’re running a second validator, make sure to insert session keys for Aura/GRANDPA.

You can use `scripts/insert_session_keys.py` to submit keys via RPC, or `scripts/key_tools.py` and `subkey` to generate and then set keys via the Sudo/Session pallet.

## Faucet usage

`scripts/faucet.py` is an off-chain client that submits `Balances.transfer_keep_alive` using an encrypted key file.

Example (0.1 MODNET = `100000000000` base units):
```bash
uv run scripts/faucet.py \
  --ws ws://127.0.0.1:9944 \
  --key-file ~/.modnet/keys/treasury.json \
  --prompt \
  --to <recipient-ss58> \
  --amount 100000000000
```

Ensure the faucet account is funded and the WebSocket endpoint is reachable.

## Common Issues

- Validation panic during `TaggedTransactionQueue_validate_transaction`:
  - Ensure your faucet client uses dynamic type registry (current script does) and correct WS endpoint.
- Connection refused:
  - Make sure the node is running and listening on WS 9944; check logs.
- Conflicting ports:
  - Validator uses HTTP RPC 9933 and WS 9944 by default; avoid changing RPC to 9944.
- External access:
  - For validators, avoid external RPC. For public endpoints, run a non-validator full node and proxy via ngrok if needed.
