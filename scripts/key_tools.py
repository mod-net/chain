#!/usr/bin/env python3
"""
Key utilities for Modnet:
- Generate Aura (sr25519) and GRANDPA (ed25519) keys via `subkey`.
- Inspect/convert public keys to SS58 addresses.
- Derive multisig address (2-of-3, or any threshold) using substrate-interface.

Requirements:
- subkey installed and on PATH (from Substrate).
- Python deps (for multisig): see scripts/requirements.txt

Usage examples:
  # Generate fresh Aura and GRANDPA keypairs
  ./scripts/key_tools.py gen-all --network substrate

  # Generate Aura only
  ./scripts/key_tools.py gen --scheme sr25519 --network substrate

  # Generate GRANDPA only
  ./scripts/key_tools.py gen --scheme ed25519 --network substrate

  # Inspect a public key to SS58
  ./scripts/key_tools.py inspect --public 0x<hex> --network substrate --scheme sr25519

  # Compute multisig address (2-of-3)
  ./scripts/key_tools.py multisig --threshold 2 \
    --signer 5F3sa2TJ... --signer 5DAAnrj7... --signer 5H3K8Z... \
    --ss58-prefix 42
"""
import argparse
import json
import shutil
import subprocess
import sys
from typing import List

from rich.console import Console
from rich.json import JSON
from rich_argparse import RichHelpFormatter

console = Console()


def require_subkey():
    if not shutil.which("subkey"):
        sys.stderr.write("Error: 'subkey' not found on PATH. Install Substrate subkey tool.\n")
        sys.exit(1)


def run(cmd: List[str]) -> str:
    proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if proc.returncode != 0:
        raise RuntimeError(f"Command failed: {' '.join(cmd)}\nSTDERR:\n{proc.stderr}")
    return proc.stdout


def parse_subkey_generate(output: str) -> dict:
    # subkey generate --scheme <scheme> prints a well-known format
    # We'll extract: Secret phrase, Public key (hex), SS58 Address
    data = {
        "secret_phrase": None,
        "public_key_hex": None,
        "ss58_address": None,
    }
    for line in output.splitlines():
        line = line.strip()
        if line.lower().startswith("secret phrase"):
            # e.g., Secret phrase:      equip will roof ...
            data["secret_phrase"] = line.split(":", 1)[1].strip()
        elif line.lower().startswith("public key (hex)"):
            data["public_key_hex"] = line.split(":", 1)[1].strip()
        elif line.lower().startswith("ss58 address"):
            data["ss58_address"] = line.split(":", 1)[1].strip()
    return data


def subkey_generate(scheme: str, network: str) -> dict:
    require_subkey()
    out = run(["subkey", "generate", "--scheme", scheme, "--network", network])
    return parse_subkey_generate(out)


def subkey_inspect(public_hex: str, network: str, scheme: str) -> dict:
    require_subkey()
    # subkey inspect --network substrate --public --scheme sr25519 0x<hex>
    out = run(["subkey", "inspect", "--network", network, "--public", "--scheme", scheme, public_hex])
    return parse_subkey_generate(out)


def multisig_address(signers: List[str], threshold: int, ss58_prefix: int) -> dict:
    try:
        from substrateinterface.utils.ss58 import ss58_encode, ss58_decode
        from scalecodec.base import ScaleBytes
        from hashlib import blake2b
    except Exception as e:
        sys.stderr.write("Error: Python deps missing. Install from scripts/requirements.txt\n")
        raise

    # The multisig account id in pallet-multisig is constructed deterministically from sorted signers and threshold.
    # Reference (pallet-multisig): multi_account_id = AccountId::from(blake2_256(b"modlpy/utilisig" ++ sorted_signers ++ threshold LE));
    # We implement the same here to ensure exact match.
    tag = b"modlpy/utilisig"

    # Decode SS58 to raw pubkey bytes (AccountId32)
    signer_pubkeys = [bytes.fromhex(ss58_decode(s)) for s in signers]
    # Sort lexicographically as per pallet
    signer_pubkeys.sort()

    # threshold as little endian u16
    thr_le = threshold.to_bytes(2, byteorder="little")

    h = blake2b(digest_size=32)
    h.update(tag)
    for pk in signer_pubkeys:
        h.update(pk)
    h.update(thr_le)
    account_id = h.digest()

    address = ss58_encode(account_id.hex(), ss58_format=ss58_prefix)
    return {"account_id_hex": account_id.hex(), "ss58_address": address}


def _print_json(obj: dict):
    # If stdout is a TTY, use rich pretty JSON; otherwise raw JSON for piping
    if sys.stdout.isatty():
        console.print(JSON.from_data(obj))
    else:
        sys.stdout.write(json.dumps(obj, indent=2) + "\n")


def cmd_gen(args):
    data = subkey_generate(args.scheme, args.network)
    _print_json({"scheme": args.scheme, "network": args.network, **data})


def cmd_gen_all(args):
    aura = subkey_generate("sr25519", args.network)
    grandpa = subkey_generate("ed25519", args.network)
    _print_json({"aura": aura, "grandpa": grandpa, "network": args.network})


def cmd_inspect(args):
    data = subkey_inspect(args.public, args.network, args.scheme)
    _print_json({"scheme": args.scheme, "network": args.network, **data})


def cmd_multisig(args):
    res = multisig_address(args.signer, args.threshold, args.ss58_prefix)
    _print_json({"threshold": args.threshold, "ss58_prefix": args.ss58_prefix, **res, "signers": args.signer})


class HelpOnErrorParser(argparse.ArgumentParser):
    def error(self, message):
        console.print(f"[red]Error:[/red] {message}")
        self.print_help()
        self.exit(2)


def main():
    p = HelpOnErrorParser(description="Key tools for Modnet", formatter_class=RichHelpFormatter)
    sub = p.add_subparsers(dest="command")

    p_gen = sub.add_parser("gen", help="Generate a single keypair via subkey")
    p_gen.add_argument("--scheme", choices=["sr25519", "ed25519"], required=True)
    p_gen.add_argument("--network", default="substrate")
    p_gen.set_defaults(func=cmd_gen)

    p_gen_all = sub.add_parser("gen-all", help="Generate Aura (sr25519) and GRANDPA (ed25519) keypairs")
    p_gen_all.add_argument("--network", default="substrate")
    p_gen_all.set_defaults(func=cmd_gen_all)

    p_inspect = sub.add_parser("inspect", help="Inspect a public key to SS58 address")
    p_inspect.add_argument("--public", required=True, help="0x<hex public key>")
    p_inspect.add_argument("--scheme", choices=["sr25519", "ed25519"], required=True)
    p_inspect.add_argument("--network", default="substrate")
    p_inspect.set_defaults(func=cmd_inspect)

    p_multi = sub.add_parser("multisig", help="Compute multisig address from signers and threshold")
    p_multi.add_argument("--signer", action="append", required=True, help="SS58 signer address; pass multiple --signer")
    p_multi.add_argument("--threshold", type=int, required=True)
    p_multi.add_argument("--ss58-prefix", type=int, default=42)
    p_multi.set_defaults(func=cmd_multisig)

    if len(sys.argv) == 1:
        p.print_help()
        sys.exit(2)

    args = p.parse_args()
    if not hasattr(args, "func"):
        p.print_help()
        sys.exit(2)
    try:
        args.func(args)
    except Exception as e:
        console.print(f"[red]Error:[/red] {e}")
        p.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
