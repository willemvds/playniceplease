#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME="playniceplease-build-release"
CONTAINER_NAME="playniceplease-build-release-static"
CONTAINER_WORKDIR="/playniceplease"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! podman image exists "$IMAGE_NAME"; then
    echo "Building image $IMAGE_NAME..."
    podman build -t "$IMAGE_NAME" -f "$SCRIPT_DIR/Containerfile.build_release" "$SCRIPT_DIR"
fi

echo "Building static release binary in container $CONTAINER_NAME..."
podman run --rm \
    --name "$CONTAINER_NAME" \
    -v "$(pwd):${CONTAINER_WORKDIR}:Z" \
    -w "$CONTAINER_WORKDIR" \
    "$IMAGE_NAME" \
    cargo build --release --offline --target x86_64-unknown-linux-musl

BINARY="${SCRIPT_DIR}/target/x86_64-unknown-linux-musl/release/playniceplease"

if command -v file >/dev/null 2>&1; then
    echo "--- file ---"
    file "$BINARY"
fi

if command -v ldd >/dev/null 2>&1; then
    echo "--- ldd ---"
    ldd "$BINARY" 2>&1 || true
fi

echo "Static release binary built at: ${BINARY}"
