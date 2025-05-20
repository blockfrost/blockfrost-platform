#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-v0.22.0}"
BASE="https://github.com/txpipe/dolos/releases/download/${VERSION}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)   FILE="dolos-aarch64-apple-darwin.tar.gz" ;;
  Darwin-x86_64)  FILE="dolos-x86_64-apple-darwin.tar.gz" ;;
  Linux-x86_64)   FILE="dolos-x86_64-unknown-linux-gnu.tar.gz" ;;
  Linux-aarch64)  FILE="dolos-aarch64-unknown-linux-gnu.tar.gz" ;;
  *) echo "Unsupported platform"; exit 1 ;;
esac

DEST="assets/dolos/${VERSION}"
mkdir -p "${DEST}"
curl -L "${BASE}/${FILE}" \
  | tar -xz -C "${DEST}" --strip-components=1

chmod +x "${DEST}/dolos"
