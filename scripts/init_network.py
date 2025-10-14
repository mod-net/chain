#!/usr/bin/env uv run
# /// script
# requires-python = ">=3.13"
# dependencies = [
#     "cryptography",
#     "pydantic",
#     "rich",
#     "rich-argparse",
#     "scalecodec",
#     "substrate-interface",
# ]
# ///
"""
Initialize Modnet network specs (testnet/mainnet) by generating a base chainspec
and patching authorities, sudo, telemetry endpoint, and boot nodes. Produces both
plain and raw JSON specs.

Usage examples:
  # Minimal (SS58 values) and one bootnode
  ./scripts/init_network.py \
    --chain-id modnet-testnet \
    --aura 5Fga63pnkp2JDGudFzpdWNzq5CwNgbS8EUTT36DKzKJi8L7p \
    --grandpa 5HF6Koc628YWoAreCmaswgesyAdcVi1MyixPbNQEz4M3xpDm \
    --sudo 5Hd73Uok5LveTaMdZTnCJDKFCUzBoP5oTgYBhZgukhvnDGEn \
    --bootnode /ip4/24.83.27.62/tcp/30333/p2p/12D3KooWHuZniGmiW8UBEdHCqp1YwA4CeCprscZcgd7n8HwVhB7s

  # Compute sudo from signers (2-of-3)
  ./scripts/init_network.py \
    --chain-id modnet-testnet \
    --aura 5F... \
    --grandpa 5H... \
    --signer 5G... --signer 5F... --signer 5F2... \
    --threshold 2 \
    --bootnode /ip4/1.2.3.4/tcp/30333/p2p/12D3...

Notes:
- Requires the modnet-node binary built at target/release/modnet-node
- Writes modnet-<id>.json and modnet-<id>-raw.json in repo root by default.
"""
import argparse
import json
import subprocess
import sys
from pathlib import Path
import re

from rich import print
from rich.console import Console

console = Console()
ROOT = Path(__file__).resolve().parents[1]
NODE_BIN = ROOT / "target" / "release" / "modnet-node"


def multisig_address(signers: list[str], threshold: int, ss58_prefix: int = 42) -> str:
    from substrateinterface.utils.ss58 import ss58_encode, ss58_decode
    from hashlib import blake2b

    tag = b"modlpy/utilisig"
    signer_pubkeys = [bytes.fromhex(ss58_decode(s)) for s in signers]
    signer_pubkeys.sort()
    thr_le = threshold.to_bytes(2, "little")
    h = blake2b(digest_size=32)
    h.update(tag)
    for pk in signer_pubkeys:
        h.update(pk)
    h.update(thr_le)
    acct = h.digest()
    return ss58_encode(acct.hex(), ss58_format=ss58_prefix)


def run(cmd: list[str]):
    proc = subprocess.run(cmd, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if proc.returncode != 0:
        raise RuntimeError(f"Command failed: {' '.join(cmd)}\nSTDERR:\n{proc.stderr}")
    return proc.stdout


def main():
    p = argparse.ArgumentParser(description="Initialize Modnet chainspecs")
    p.add_argument("--chain-id", default="modnet-testnet", help="ChainSpec id to build (modnet-testnet)")
    p.add_argument("--out-prefix", default="modnet-testnet", help="Output prefix for files")

    p.add_argument("--aura", required=True, help="Aura authority (SS58 or 0x hex)")
    p.add_argument("--grandpa", required=True, help="GRANDPA authority (SS58 or 0x hex)")

    g = p.add_mutually_exclusive_group(required=True)
    g.add_argument("--sudo", help="Sudo SS58 (e.g., multisig)")
    g.add_argument("--signer", action="append", help="Signers SS58 to compute multisig; pass multiple")
    p.add_argument("--threshold", type=int, help="Multisig threshold if using --signer", default=None)

    p.add_argument("--bootnode", action="append", default=[], help="Bootnode multiaddr; pass multiple")
    p.add_argument("--telemetry", default=None, help="Override telemetry submit URL (e.g., wss://telemetry.polkadot.io/submit/)")

    args = p.parse_args()

    if not NODE_BIN.exists():
        console.print(f"[red]Missing binary[/red]: {NODE_BIN}. Build first: cargo build --release")
        sys.exit(1)

    # Ensure output directory exists
    specs_dir = ROOT / "specs"
    specs_dir.mkdir(parents=True, exist_ok=True)
    plain = specs_dir / f"{args.out_prefix}.json"
    raw = specs_dir / f"{args.out_prefix}-raw.json"

    console.print(f"[cyan]Building base spec for[/cyan] {args.chain_id} -> {plain}")
    plain_text = run([str(NODE_BIN), "build-spec", "--chain", args.chain_id])
    with plain.open("w") as f:
        f.write(plain_text)

    # Load, patch, write
    with plain.open("r") as f:
        spec = json.load(f)

    # Navigate to patch root
    patch = spec.get("genesis", {}).get("runtimeGenesis", {}).get("patch", {})
    if not patch:
        console.print("[red]Unexpected chainspec layout; cannot find runtimeGenesis.patch[/red]")
        sys.exit(1)

    # Accept SS58 or 0x-hex public keys. If hex, convert to SS58 using subkey.
    def _to_ss58(val: str, scheme: str) -> str:
        s = (val or "").strip()
        if s.startswith("0x"):
            out = run([str(NODE_BIN), "subkey", "inspect", "--public", "--scheme", scheme, s]) if False else None
            # modnet-node may not expose subkey; use standalone subkey on PATH instead
            try:
                subkey_out = subprocess.run(["subkey", "inspect", "--network", "substrate", "--public", "--scheme", scheme, s], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                if subkey_out.returncode != 0:
                    raise RuntimeError(subkey_out.stderr)
                # parse SS58 Address line
                ss58 = None
                for line in subkey_out.stdout.splitlines():
                    if line.lower().startswith("ss58 address"):
                        ss58 = line.split(":", 1)[1].strip()
                        break
                if not ss58:
                    raise RuntimeError("failed to parse SS58 from subkey output")
                return ss58
            except Exception as e:
                raise RuntimeError(f"Failed to convert hex to SS58 via subkey for scheme {scheme}: {e}")
        return s

    aura_ss58 = _to_ss58(args.aura, "sr25519")
    grandpa_ss58 = _to_ss58(args.grandpa, "ed25519")

    patch.setdefault("aura", {})["authorities"] = [aura_ss58]
    patch.setdefault("grandpa", {})["authorities"] = [[grandpa_ss58, 1]]

    if args.sudo:
        patch.setdefault("sudo", {})["key"] = args.sudo
    else:
        if not args.threshold:
            console.print("[red]Provide --threshold with --signer[/red]")
            sys.exit(1)
        sudo_ms = multisig_address(args.signer, args.threshold)
        console.print(f"Computed sudo multisig: [green]{sudo_ms}[/green]")
        patch.setdefault("sudo", {})["key"] = sudo_ms

    # Bootnodes: validate and filter to avoid invalid multiaddrs breaking build-spec
    if args.bootnode:
        def _valid_multiaddr(s: str) -> bool:
            s = (s or "").strip()
            # Very simple check: /ip4/<ipv4>/tcp/<port>/p2p/<peerid>
            pat = re.compile(r"^/ip4/\d+\.\d+\.\d+\.\d+/tcp/\d+/p2p/[A-Za-z0-9]+$")
            return bool(pat.match(s))

        cleaned = [b.strip() for b in args.bootnode if isinstance(b, str) and b.strip()]
        valid = [b for b in cleaned if _valid_multiaddr(b)]
        skipped = [b for b in cleaned if b not in valid]
        if skipped:
            console.print(f"[yellow]Skipping invalid bootnode multiaddr(s):[/yellow] {skipped}")
        if valid:
            spec["bootNodes"] = valid

    # Telemetry
    if args.telemetry:
        spec.setdefault("telemetryEndpoints", {"endpoints": []})
        spec["telemetryEndpoints"]["endpoints"] = [[args.telemetry, 0]]

    with plain.open("w") as f:
        json.dump(spec, f, indent=2)

    console.print(f"[cyan]Building raw spec ->[/cyan] {raw}")
    raw_text = run([str(NODE_BIN), "build-spec", "--chain", str(plain), "--raw"])
    with raw.open("w") as f:
        f.write(raw_text)
    console.print("[green]Done.[/green]")


if __name__ == "__main__":
    main()
