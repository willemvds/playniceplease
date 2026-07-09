#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME="playniceplease-build-release"
CONTAINER_NAME="playniceplease-build-release"
CONTAINER_WORKDIR="/playniceplease"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! podman image exists "$IMAGE_NAME"; then
    echo "Building image $IMAGE_NAME..."
    podman build -t "$IMAGE_NAME" -f "$SCRIPT_DIR/Containerfile.build_release" "$SCRIPT_DIR"
fi

echo "Building release binary in container $CONTAINER_NAME..."
podman run --rm \
    --name "$CONTAINER_NAME" \
    -v "$(pwd):${CONTAINER_WORKDIR}:Z" \
    -w "$CONTAINER_WORKDIR" \
    "$IMAGE_NAME"

BINARY="${SCRIPT_DIR}/target/release/playniceplease"
echo "Release binary built at: ${BINARY}"
