# Keystore Security Hardening for Validator Nodes

## Background
Substrate stores session keys (AURA/GRANDPA) in the local keystore on disk under the node data path. These keys are not encrypted at rest by default. If an attacker obtains them, they can impersonate the validator and cause equivocation and slashing events.

## Risks
- Impersonation of validator (block production/finality votes)
- Slashing due to equivocation
- Censorship/disruption from compromised validator
- Reputational damage and potential chain instability

## Goals
- Reduce likelihood and impact of keystore compromise.
- Establish repeatable, automated operational practices across environments.

## Scope
- Node runtime and host/container hardening for testnet and mainnet.
- Operational procedures for key rotation, insertion, and recovery.

## Out of Scope
- Application-layer wallet/account management (treasury, sudo, etc.).
- HSM integration (can be evaluated later).

---

## Proposed Changes

1. File-system hardening
- Create dedicated data path per node, e.g. `/var/lib/modnet/node{N}` (native) or a dedicated docker volume.
- Ensure ownership by a dedicated unprivileged user (e.g., `modnet`) and restrictive permissions.
- If feasible, place the data path on an encrypted volume (LUKS or cloud volume encryption).

2. Node startup flags and scripts
- Always set explicit `--base-path`.
- Disable unsafe RPC in non-test environments; only enable when needed.
- Recommended flags per environment:
  - Testnet:
    - `--rpc-external --rpc-cors=all --rpc-methods=Unsafe` (only when inserting keys; otherwise remove `Unsafe`).
  - Mainnet:
    - No external RPC or `Unsafe` methods; use controlled access or temporarily enable during maintenance windows.
- Update `scripts/start_node.sh` to:
  - Accept `--base-path` and RPC-related flags via env vars.
  - Create the base path with secure permissions if missing.

3. Docker-compose adjustments (if applicable)
- Separate volumes for node1/node2 data.
- Parameterize RPC exposure and methods via environment variables (`RPC_METHODS`, `RPC_EXTERNAL`, `RPC_CORS`).
- Provide a `purge-and-restart` make/docker-compose target to safely tear down and start from fresh genesis.

4. Key management flow
- Generate keys via `scripts/key_tools.py` (password-protected, scrypt+AES-GCM), store encrypted artifacts in `~/.modnet/keys`.
- Insert into keystore using `scripts/insert_session_keys.py` only when node RPC is in a controlled/secure mode.
- After key insertion, disable `Unsafe` RPC methods.
- Rotate session keys regularly (e.g., per era or scheduled cadence):
  - Generate new keys, insert with `author_insertKey`, submit `session.setKeys` extrinsic, wait for activation window.

5. Backup and recovery
- Never back up raw keystore files.
- Back up only the encrypted key files (`~/.modnet/keys/*.json`) and their passwords via a secret manager.
- Document disaster recovery steps: stop node, rotate keys, reinsert, call `session.setKeys`.

6. Monitoring and incident response
- Alerts for node RPC exposure and `Unsafe` methods enabled unexpectedly.
- Watch for equivocation reports and validator behavior anomalies.
- Incident runbook: immediate stop, revoke/rotate keys, rejoin after cooldown.

7. Documentation
- Add a "Keystore Security" section to `README.md` summarizing the above.
- Keep the `docs/session-keys.json` manifest format for recording public addresses and file paths (no secrets).

---

## Implementation Plan

- [ ] Update `scripts/start_node.sh`:
  - [ ] Support `BASE_PATH`, `RPC_EXTERNAL`, `RPC_METHODS`, `RPC_CORS` environment variables.
  - [ ] `mkdir -p "$BASE_PATH"` and set permissions (`chmod 700`).
  - [ ] Pass `--base-path "$BASE_PATH"` and conditional RPC flags.
- [ ] (Optional) Update `docker-compose.yml`:
  - [ ] Add volumes for `node1_data`, `node2_data`.
  - [ ] Expose RPC only when `RPC_METHODS=Unsafe` is explicitly set.
  - [ ] Provide a `make purge-and-restart` or compose target.
- [ ] Add `README.md` section for Keystore Security and operations.
- [ ] Add a `scripts/session_set_keys.py` helper to submit the `session.setKeys` extrinsic CLI-side.
- [ ] Add CI lint to prevent committing raw keystore paths or files.

---

## Acceptance Criteria
- `scripts/start_node.sh` can start a node with a specified `--base-path` created with secure permissions.
- In non-test runs, RPC does not expose `Unsafe` methods by default.
- Documented command(s) to:
  - Generate & encrypt keys (`key_tools.py`).
  - Insert keys via RPC in a controlled mode (`insert_session_keys.py`).
  - Submit `session.setKeys` on-chain for each validator.
- `README.md` contains the Keystore Security section listing risks and mitigations.
- (If using compose) Data volumes are separated per node and a purge and restart workflow is documented.

---

## References
- Substrate RPC: `author_insertKey`
- Session keys and rotation via `session.setKeys`
- Substrate keystore location and semantics
