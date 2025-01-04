#!/bin/bash

set -e

OUT_DIR="out"
IMG_PROMPT="$1"
if [ -z "$IMG_PROMPT" ] ; then
  read -p "Image Prompt: " IMG_PROMPT
fi
TEXT_PROMPT="$2"
if [ -z "$TEXT_PROMPT" ] ; then
  read -p "Text Prompt: " TEXT_PROMPT
fi

# Yes, we swap compile commands based on plugged-in hardware.
if lspci -k -d ::03xx | grep -i nvidia ; then
  echo "Building with CUDA support"

  cargo build --release \
    --no-default-features \
    --features oliana_gui/wayland \
    --features oliana_images/cuda \
    --features oliana_text/cuda

  echo "Built with CUDA support!"
else
  echo "Building w/o CUDA support"

  cargo build \
    --release \
    --no-default-features \
    --features oliana_gui/wayland

  echo "Built w/o CUDA support!"
fi

pkill oliana || true
pkill oliana || true

./target/release/oliana_server &
SERVER_PID=$!

for i in $(seq 0 30) ; do
  echo "[ test-server-text-and-image-gen.sh ] Waiting for server to come online..."
  sleep 1
done

mkdir -p $OUT_DIR

echo "Testing Image generation with prompt '$IMG_PROMPT', results are going in the folder $OUT_DIR"

./target/release/oliana_client image \
  --prompt "$IMG_PROMPT" \
  --output "$OUT_DIR/0.png"

./target/release/oliana_client image \
  --prompt "$IMG_PROMPT" \
  --output "$OUT_DIR/1.png"

./target/release/oliana_client image \
  --prompt "$IMG_PROMPT" \
  --output "$OUT_DIR/2.png"

echo "Testing Text generation with prompt '$TEXT_PROMPT', results are going in the folder $OUT_DIR"

./target/release/oliana_client text \
  --prompt "$TEXT_PROMPT" \
  --output "$OUT_DIR/0.txt"

./target/release/oliana_client text \
  --prompt "$TEXT_PROMPT" \
  --output "$OUT_DIR/1.txt"

./target/release/oliana_client text \
  --prompt "$TEXT_PROMPT" \
  --output "$OUT_DIR/2.txt"

echo "Done!"

# Send SIGTERM to process
kill -15 $SERVER_PID || true

sleep 2
# Kill it
kill -9 $SERVER_PID || true
