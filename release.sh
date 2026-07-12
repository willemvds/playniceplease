#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 VERSION" >&2
    exit 1
fi

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

RELEASE_DIR="${SCRIPT_DIR}/releases/playniceplease-x86_64-linux-${VERSION}"
mkdir -p "$RELEASE_DIR"
cp -f "$BINARY" "${RELEASE_DIR}/playniceplease"
echo "Released ${VERSION} to: ${RELEASE_DIR}/playniceplease"
