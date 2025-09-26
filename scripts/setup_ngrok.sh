#!/usr/bin/env bash
set -euo pipefail

NODE_TYPE=$1
NODE_NUMBER=$2

sed -e '/^endpoints:/,/protocol: http1$/c\
  - name: "$NODE_TYPE_$NODE_NUMBER"\
    url: "https://$NODE_TYPE-$NODE_NUMBER-comai.ngrok.dev"\
    upstream:\
      url:993"$NODE_NUMBER"\
      protocol: http1
' ~/.config/ngrok/ngrok.yml > ~/.config/ngrok/ngrok.new.yml