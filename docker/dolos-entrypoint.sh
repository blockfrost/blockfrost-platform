#!/bin/sh

set -eu

SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"

"$SCRIPT_DIR/generate-dolos-config.sh" "$NETWORK" >/config.toml

cd /data/

# We bootstrap from Mithril, because it's safer.
# `/data/snapshot` will be cleared once the bootstrap process finishes.
test -e /data/chain || dolos --config /config.toml bootstrap mithril --download-dir /data/snapshot

exec dolos --config /config.toml daemon
