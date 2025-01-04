#!/bin/bash

set -e

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

if [ "$1" = "norun" ] || [ "$1" = "build" ]; then
  exit 0
fi

./target/release/oliana_gui || pkill oliana

