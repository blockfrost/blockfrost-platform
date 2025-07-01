#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version> (e.g. v0.22.0)"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

VERSION="$1"
BASE="https://github.com/txpipe/dolos/releases/download/${VERSION}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)   FILE="dolos-aarch64-apple-darwin.tar.gz" ;;
  Darwin-x86_64)  FILE="dolos-x86_64-apple-darwin.tar.gz" ;;
  Linux-x86_64)   FILE="dolos-x86_64-unknown-linux-gnu.tar.gz" ;;
  Linux-aarch64)  FILE="dolos-aarch64-unknown-linux-gnu.tar.gz" ;;
  *) echo "Unsupported platform"; exit 1 ;;
esac

DEST="${SCRIPT_DIR}/../bin/${VERSION}"
mkdir -p "${DEST}"
curl -L "${BASE}/${FILE}" \
  | tar -xz -C "${DEST}" --strip-components=1

chmod +x "${DEST}/dolos"
