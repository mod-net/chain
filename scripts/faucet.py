#!/usr/bin/env python3
"""
Simple faucet for Modnet.

- Decrypts a key file created by key_tools.py
- Connects to a node over WebSocket (local or ngrok)
- Sends balances.transferKeepAlive extrinsics
- Optional rate limiting by recipient address

Examples:
  ./scripts/faucet.py \
    --ws wss://chain-rpc-comai.ngrok.dev \
    --key-file ~/.modnet/keys/aura-primary.json \
    --prompt \
    --to 5F... --amount 100

  # Run as a small service from CLI (one at a time)
  ./scripts/faucet.py --ws ws://127.0.0.1:9944 --key-file ~/.modnet/keys/faucet.json --prompt --to 5F... --amount 10

Notes:
- Amount is in Modnet base units (like Plancks) as integer.
- Make sure the faucet account is endowed.
- For production, consider adding CAPTCHA and stronger rate limits.
"""
import argparse
import os
import sys
import time
from typing import Optional

from substrateinterface import SubstrateInterface, Keypair
from substrateinterface.exceptions import SubstrateRequestException

# Reuse our key loading
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)
from key_tools import Key  # type: ignore


def load_faucet_key(path: str, password: Optional[str]) -> Keypair:
    key = Key.load(os.path.expanduser(path), password)
    if not key.secret_phrase:
        raise RuntimeError("Key file did not contain a mnemonic (secret_phrase)")
    return Keypair.create_from_mnemonic(key.secret_phrase, ss58_format=42)


def connect(websocket_url: str) -> SubstrateInterface:
    # Do not hardcode a type registry preset; let substrate-interface fetch metadata/types from the chain.
    # A mismatched preset can lead to runtime panics in TaggedTransactionQueue_validate_transaction.
    return SubstrateInterface(url=websocket_url, ss58_format=42)


def transfer(substrate: SubstrateInterface, faucet_keypair: Keypair, recipient_address: str, amount_base_units: int) -> str:
    call = substrate.compose_call(
        call_module='Balances',
        call_function='transfer_keep_alive',
        call_params={'dest': recipient_address, 'value': amount_base_units},
    )
    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=faucet_keypair)
    receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
    return receipt.extrinsic_hash


def main():
    parser = argparse.ArgumentParser(description='Modnet Faucet')
    parser.add_argument('--ws', required=True, help='WebSocket endpoint, e.g., ws://127.0.0.1:9944 or wss://...')
    parser.add_argument('--key-file', required=True, help='Encrypted key file path for faucet account')
    parser.add_argument('--password', help='Password for key file (omit to prompt)')
    parser.add_argument('--prompt', action='store_true', help='Prompt for password')
    parser.add_argument('--to', required=True, help='Recipient SS58 address')
    parser.add_argument('--amount', required=False, type=int, default=1000, help='Amount in base units (integer). Default: 1000')
    parser.add_argument('--sleep', type=float, default=0.0, help='Sleep seconds after sending (for simple throttling)')

    parsed_args = parser.parse_args()

    faucet_password = None if parsed_args.prompt else parsed_args.password
    faucet_keypair = load_faucet_key(parsed_args.key_file, faucet_password)
    substrate = connect(parsed_args.ws)

    try:
        tx_hash = transfer(substrate, faucet_keypair, parsed_args.to, parsed_args.amount)
        print({'status': 'ok', 'tx_hash': tx_hash})
    except SubstrateRequestException as e:
        print({'status': 'error', 'error': str(e)})
        sys.exit(1)
    finally:
        if parsed_args.sleep > 0:
            time.sleep(parsed_args.sleep)


if __name__ == '__main__':
    main()
