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

  # Derive public SS58 address from secret phrase
  ./scripts/key_tools.py derive --secret <phrase> --scheme sr25519 --network substrate

  # Save key to file (encrypted)
  ./scripts/key_tools.py key-save --secret <phrase> --scheme sr25519 --network substrate --path /tmp/key.json --password <password>

  # Load key from file (encrypted)
  ./scripts/key_tools.py key-load --path /tmp/key.json --password <password>
"""
import argparse
import json
import shutil
import subprocess
import sys
from datetime import datetime
from typing import List

from rich.console import Console
from rich.json import JSON
from rich_argparse import RichHelpFormatter
from pydantic import BaseModel, ConfigDict

console = Console()


class Key(BaseModel):
    model_config = ConfigDict(frozen=True)
    scheme: str
    network: str
    byte_array: bytes
    mnemonic_phrase: str | None = None
    secret_phrase: str | None = None
    public_key_hex: str
    private_key_hex: str | None = None
    ss58_address: str | None = None
    key_type: Literal["sr25519", "ed25519", "ss58"] | None = None
    is_pair: bool = False
    is_multisig: bool = False
    threshold: int | None = None
    signers: List[str] | None = None
    multisig_address: str | None = None
    created_at: datetime | None = None
    

class AuraSR25519Key(Key):
    secret_phrase: str

class GrandpaED25519Key(Key):
    secret_phrase: str

class ModNetSS58Key(Key):
    public_key_hex: str
    ss58_address: str


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

# -----------------------------
# Key object + crypto helpers
# -----------------------------
from dataclasses import dataclass, asdict
from getpass import getpass
import os
import base64
from typing import Optional

try:
    from cryptography.hazmat.primitives.kdf.scrypt import Scrypt
    from cryptography.hazmat.primitives.ciphers.aead import AESGCM
    _CRYPTO_OK = True
except Exception:
    _CRYPTO_OK = False


def _require_crypto():
    if not _CRYPTO_OK:
        raise RuntimeError("Missing crypto deps. Install with: pip install -r scripts/requirements.txt")


def _kdf_scrypt(password: str, salt: bytes, n: int = 2**14, r: int = 8, p: int = 1, length: int = 32) -> bytes:
    kdf = Scrypt(salt=salt, length=length, n=n, r=r, p=p)
    return kdf.derive(password.encode("utf-8"))


def _aesgcm_encrypt(key: bytes, plaintext: bytes, aad: bytes = b"") -> dict:
    nonce = os.urandom(12)
    aes = AESGCM(key)
    ct = aes.encrypt(nonce, plaintext, aad)
    return {"nonce": base64.b64encode(nonce).decode(), "ciphertext": base64.b64encode(ct).decode()}


def _aesgcm_decrypt(key: bytes, nonce_b64: str, ciphertext_b64: str, aad: bytes = b"") -> bytes:
    nonce = base64.b64decode(nonce_b64)
    ct = base64.b64decode(ciphertext_b64)
    aes = AESGCM(key)
    return aes.decrypt(nonce, ct, aad)


@dataclass
class Key:
    scheme: str
    network: str = "substrate"
    secret_phrase: Optional[str] = None
    public_key_hex: Optional[str] = None
    ss58_address: Optional[str] = None

    @staticmethod
    def from_secret_phrase(phrase: str, scheme: str, network: str = "substrate") -> "Key":
        require_subkey()
        # subkey inspect will print public + ss58 for the phrase
        out = run(["subkey", "inspect", "--scheme", scheme, "--network", network, phrase])
        parsed = parse_subkey_generate(out)
        return Key(
            scheme=scheme,
            network=network,
            secret_phrase=phrase,
            public_key_hex=parsed.get("public_key_hex"),
            ss58_address=parsed.get("ss58_address"),
        )

    @staticmethod
    def from_public(public_hex: str, scheme: str, network: str = "substrate") -> "Key":
        parsed = subkey_inspect(public_hex, network, scheme)
        return Key(
            scheme=scheme,
            network=network,
            public_key_hex=public_hex,
            ss58_address=parsed.get("ss58_address"),
        )

    def derive_public_ss58(self) -> "Key":
        if self.public_key_hex and self.ss58_address:
            return self
        if self.secret_phrase:
            derived = Key.from_secret_phrase(self.secret_phrase, self.scheme, self.network)
            self.public_key_hex = derived.public_key_hex
            self.ss58_address = derived.ss58_address
            return self
        raise ValueError("No data available to derive from; provide secret_phrase or public_key_hex")

    def to_json(self, include_secret: bool = False) -> dict:
        data = {
            "scheme": self.scheme,
            "network": self.network,
            "public_key_hex": self.public_key_hex,
            "ss58_address": self.ss58_address,
        }
        if include_secret:
            data["secret_phrase"] = self.secret_phrase
        return data

    # Encryption format: JSON with scrypt params, salt, nonce, ciphertext (base64)
    def encrypt(self, password: str) -> dict:
        _require_crypto()
        payload = json.dumps(self.to_json(include_secret=True)).encode()
        salt = os.urandom(16)
        key = _kdf_scrypt(password, salt)
        enc = _aesgcm_encrypt(key, payload)
        return {
            "version": 1,
            "kdf": "scrypt",
            "salt": base64.b64encode(salt).decode(),
            "params": {"n": 16384, "r": 8, "p": 1},
            **enc,
        }

    @staticmethod
    def decrypt(blob: dict, password: str) -> "Key":
        _require_crypto()
        if blob.get("kdf") != "scrypt":
            raise ValueError("Unsupported KDF")
        params = blob.get("params") or {}
        n, r, p = params.get("n", 16384), params.get("r", 8), params.get("p", 1)
        salt = base64.b64decode(blob["salt"]) if isinstance(blob.get("salt"), str) else blob.get("salt")
        key = _kdf_scrypt(password, salt, n=n, r=r, p=p)
        pt = _aesgcm_decrypt(key, blob["nonce"], blob["ciphertext"])  # type: ignore
        data = json.loads(pt.decode())
        return Key(
            scheme=data["scheme"],
            network=data.get("network", "substrate"),
            secret_phrase=data.get("secret_phrase"),
            public_key_hex=data.get("public_key_hex"),
            ss58_address=data.get("ss58_address"),
        )

    def save(self, path: str, password: Optional[str] = None) -> None:
        if password is None:
            pw1 = getpass("Set password for key file: ")
            pw2 = getpass("Confirm password: ")
            if pw1 != pw2:
                raise ValueError("Passwords do not match")
            password = pw1
        blob = self.encrypt(password)
        with open(path, "w") as f:
            json.dump(blob, f, indent=2)

    @staticmethod
    def load(path: str, password: Optional[str] = None) -> "Key":
        if password is None:
            password = getpass("Password for key file: ")
        with open(path, "r") as f:
            blob = json.load(f)
        return Key.decrypt(blob, password)


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


def cmd_derive(args):
    if args.phrase:
        k = Key.from_secret_phrase(args.phrase, args.scheme, args.network)
    elif args.public:
        k = Key.from_public(args.public, args.scheme, args.network)
    else:
        raise ValueError("Provide --phrase or --public")
    k = k.derive_public_ss58()
    _print_json(k.to_json(include_secret=args.with_secret))


def cmd_key_save(args):
    if args.phrase:
        k = Key.from_secret_phrase(args.phrase, args.scheme, args.network)
    elif args.public:
        k = Key.from_public(args.public, args.scheme, args.network)
    else:
        raise ValueError("Provide --phrase or --public")
    k.save(args.out, None if args.prompt else args.password)
    console.print(f"[green]Saved encrypted key to[/green] {args.out}")


def cmd_key_load(args):
    k = Key.load(args.file, None if args.prompt else args.password)
    _print_json(k.to_json(include_secret=args.with_secret))


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

    p_derive = sub.add_parser("derive", help="Derive public/SS58 from a secret phrase or public key")
    p_derive.add_argument("--scheme", choices=["sr25519", "ed25519"], required=True)
    p_derive.add_argument("--network", default="substrate")
    p_derive.add_argument("--phrase", help="Secret phrase (mnemonic)")
    p_derive.add_argument("--public", help="0x<hex public key>")
    p_derive.add_argument("--with-secret", action="store_true", help="Include secret in output (if available)")
    p_derive.set_defaults(func=cmd_derive)

    p_save = sub.add_parser("key-save", help="Encrypt and save a key file (scrypt+AES-GCM)")
    p_save.add_argument("--scheme", choices=["sr25519", "ed25519"], required=True)
    p_save.add_argument("--network", default="substrate")
    p_save.add_argument("--phrase", help="Secret phrase (mnemonic)")
    p_save.add_argument("--public", help="0x<hex public key>")
    p_save.add_argument("--out", required=True, help="Output file path")
    p_save.add_argument("--password", help="Password (omit to be prompted)")
    p_save.add_argument("--prompt", action="store_true", help="Prompt for password (recommended)")
    p_save.set_defaults(func=cmd_key_save)

    p_load = sub.add_parser("key-load", help="Decrypt a saved key file and print fields")
    p_load.add_argument("--file", required=True)
    p_load.add_argument("--password", help="Password (omit to be prompted)")
    p_load.add_argument("--prompt", action="store_true", help="Prompt for password")
    p_load.add_argument("--with-secret", action="store_true", help="Include secret in output")
    p_load.set_defaults(func=cmd_key_load)

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
