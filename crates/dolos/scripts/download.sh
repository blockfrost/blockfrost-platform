#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version> [<dest_dir>]"
  echo "  e.g. $0 v0.22.0 /absolute/path/to/target/dolos/bin/v0.22.0"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="$1"

# If you passed a second argument, use it; otherwise default back to ../bin/<version>
if [[ -n "${2-}" ]]; then
  DEST="$2"
else
  DEST="${SCRIPT_DIR}/../bin/${VERSION}"
fi

# Build the download URL
BASE="https://github.com/txpipe/dolos/releases/download/${VERSION}"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)   FILE="dolos-aarch64-apple-darwin.tar.gz" ;;
  Darwin-x86_64)  FILE="dolos-x86_64-apple-darwin.tar.gz" ;;
  Linux-x86_64)   FILE="dolos-x86_64-unknown-linux-gnu.tar.gz" ;;
  Linux-aarch64)  FILE="dolos-aarch64-unknown-linux-gnu.tar.gz" ;;
  *) echo "Unsupported platform"; exit 1 ;;
esac

# Ensure the destination directory exists
mkdir -p "${DEST}"

# Stream download → extract into DEST, stripping top-level directory
echo "Downloading dolos ${VERSION} → ${DEST}/${FILE}"
curl --fail --location "${BASE}/${FILE}" \
  | tar -xz -C "${DEST}" --strip-components=1

# Make the binary executable
chmod +x "${DEST}/dolos"

echo "✓ Installed dolos ${VERSION} to ${DEST}/dolos"
