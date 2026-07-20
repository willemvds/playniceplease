/!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 VERSION" >&2
    exit 1
fi

BINARY_NAME="playnicepls"
IMAGE_NAME="playnice_please-build-release"
CONTAINER_NAME="playnice_please-build-release"
CONTAINER_WORKDIR="/playnice_please"
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

BINARY="${SCRIPT_DIR}/target/release/playnicepls"
echo "Release binary built at: ${BINARY}"

RELEASE_DIR="${SCRIPT_DIR}/releases/playnice_please-x86_64-linux-${VERSION}"
mkdir -p "$RELEASE_DIR"
cp -f "$BINARY" "${RELEASE_DIR}/${BINARY_NAME}"
echo "Released ${VERSION} to: ${RELEASE_DIR}/${BINARY_NAME}"

ZIP_NAME="$(basename "${RELEASE_DIR}").zip"
zip -j "${RELEASE_DIR}/${ZIP_NAME}" "${RELEASE_DIR}/${BINARY_NAME}"
mv -f "${RELEASE_DIR}/${ZIP_NAME}" "${SCRIPT_DIR}/releases/"
echo "Released archive: ${SCRIPT_DIR}/releases/${ZIP_NAME}"
