#!/usr/bin/env bash
set -euo pipefail

NODE_TYPE=$1
NODE_NUMBER=$2

CONFIG="$HOME/.config/ngrok/ngrok.yml"
NEWCONFIG="$HOME/.config/ngrok/ngrok.new.yml"

# Build new endpoint block
TEMPLATE="endpoints:
  - name: \"${NODE_TYPE}_${NODE_NUMBER}\"
    url: https://${NODE_TYPE}-${NODE_NUMBER}-comai.ngrok.dev
    upstream:
      url: 993${NODE_NUMBER}
      protocol: http1"

# Copy everything before the first 'endpoints:' line into the new file
awk '/^endpoints:/ {exit} {print}' "$CONFIG" > "$NEWCONFIG"

# Append our new endpoint block
echo "$TEMPLATE" >> "$NEWCONFIG"

# Show result
cat "$NEWCONFIG"

# Confirm with user
read -p "Confirm change (Y/n): " confirm
confirm=$(echo "${confirm,,}")   # lowercase input

if [[ -z "$confirm" || "$confirm" == y* ]]; then
  mv "$NEWCONFIG" "$CONFIG"
  echo "Config updated."
else
  echo "Aborted."
  rm "$NEWCONFIG"
  exit 1
fi

read -p "Start ngrok?(Y/n): " confirm
confirm=$(echo "${confirm,,}")   # lowercase input

if [[ -z "$confirm" || "$confirm" == y* ]]; then
  pm2 start 'ngrok start --all' -n ngrok
fi
