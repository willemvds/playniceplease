#!/usr/bin/env bash
set -euo pipefail

REPO="willemvds/playniceplease"
VERSION="0.1.2"
ARCH="x86_64"
ASSET="playniceplease-${ARCH}-linux-static-${VERSION}.zip"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET}"
INSTALL_DIR="/usr/bin"
BINARY_NAME="playniceplease"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

echo "Downloading ${ASSET}..."
if ! curl -fsSL -o "${tmpdir}/${ASSET}" "${DOWNLOAD_URL}"; then
    echo "Error: failed to download ${DOWNLOAD_URL}" >&2
    exit 2
fi

echo "Extracting ${ASSET}..."
if ! unzip -o "${tmpdir}/${ASSET}" -d "${tmpdir}" >/dev/null; then
    echo "Error: failed to extract ${ASSET}" >&2
    exit 2
fi

if [[ ! -x "${tmpdir}/${BINARY_NAME}" ]]; then
    echo "Error: ${BINARY_NAME} not found in archive or not executable" >&2
    exit 2
fi

echo "Installing ${BINARY_NAME} to ${INSTALL_DIR}/"
if (( EUID != 0 )); then
    if ! sudo install -m 0755 "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"; then
        echo "Error: failed to install ${BINARY_NAME} (sudo install)" >&2
        exit 2
    fi
else
    if ! install -m 0755 "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"; then
        echo "Error: failed to install ${BINARY_NAME}" >&2
        exit 2
    fi
fi

echo "Verifying installation..."
if ! command -v "${BINARY_NAME}" >/dev/null 2>&1; then
    echo "Error: ${BINARY_NAME} not found on PATH after install" >&2
    exit 2
fi

echo "Installed: $(command -v "${BINARY_NAME}")"
exit 0