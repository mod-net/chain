#!/bin/bash

set -euo pipefail

source scripts/source_keys.sh

SIGNATORY_1="5G9MCPkRmbYKvRwSog6wnfGsa474mZ7E6gyYAFjPgJMDczhq"
SIGNATORY_2="5Fqfm4drTEfBdCmjnCSTQpxWLg82UgDP3R7zKNnqFFj2GpkY"
SIGNATORY_3="5F27CcXGCpHE6ZLWV1Qy2EjNro9byxsAYzQT1kpjNwnGrguJ"

uv run scripts/init_network.py \
    --chain-id modnet-testnet \
    --aura ${SS58_AURA:-$PUBLIC_KEY_HEX_AURA} \
    --grandpa ${SS58_GRANDPA:-$PUBLIC_KEY_HEX_GRANDPA} \
    --signer ${SIGNATORY_1} \
    --signer ${SIGNATORY_2} \
    --signer ${SIGNATORY_3} \
    --threshold 2 \
    ${BOOTNODE_MULTIADDR:+--bootnode ${BOOTNODE_MULTIADDR}}