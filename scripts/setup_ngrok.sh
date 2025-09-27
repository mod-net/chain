#!/usr/bin/env bash
set -euo pipefail

NODE_TYPE=$1
NODE_NUMBER=$2

TEMPLATE="endpoints:\n\
  - name: "$NODE_TYPE"_"$NODE_NUMBER"\n\
    url: \https://"$NODE_TYPE"-"$NODE_NUMBER"-comai.ngrok.dev\n\
    upstream:\n\
      url: 993$NODE_NUMBER\n\
      protocol: http1"

sed -e "/^endpoints:/,/protocol: http1$/c$TEMPLATE" $HOME/.config/ngrok/ngrok.yml > $HOME/.config/ngrok/ngrok.yml