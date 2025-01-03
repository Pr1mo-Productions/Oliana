#!/bin/bash

set -e

git pull || true

echo "Building with CUDA support"

cargo build --release \
  --no-default-features \
  --features oliana_gui/wayland \
  --features oliana_images/cuda \
  --features oliana_text/cuda

echo "Built with CUDA support!"

sudo systemctl restart oliana-server

journalctl -f -u oliana-server

