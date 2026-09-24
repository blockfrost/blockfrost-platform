#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------- #

if [[ -z ${NETWORK:-} ]]; then
  echo NETWORK must be explicitly set
  exit 1
fi
if [[ -z ${UNPACK_DIR:-} ]]; then
  echo UNPACK_DIR must be explicitly set
  exit 1
fi

# Mithril distribution tag; keep in sync with the `mithril-client` image tag in
# `docker-compose.yml`. Used to pin the verification keys below to this exact
# release instead of tracking `main` (the client binary doesn't expose it).
MITHRIL_TAG="2630.0"

if [ -d "$UNPACK_DIR/db" ]; then
  echo "Directory $UNPACK_DIR/db exists, nothing to do. If you are having issues with your cardano node database please remove the volume and restart"
  exit 0
fi

case "$NETWORK" in
"mainnet") MITHRIL_NETWORK="release-mainnet" ;;
"preprod") MITHRIL_NETWORK="release-preprod" ;;
"preview") MITHRIL_NETWORK="pre-release-preview" ;;
*)
  echo >&2 "fatal: invalid \$NETWORK value: $NETWORK"
  exit 1
  ;;
esac
export MITHRIL_NETWORK

export AGGREGATOR_ENDPOINT="https://aggregator.${MITHRIL_NETWORK}.api.mithril.network/aggregator"

# ---------------------------------------------------------------------------- #

apt update
apt install curl -y

# ---------------------------------------------------------------------------- #

vkey_base="https://raw.githubusercontent.com/IntersectMBO/mithril/${MITHRIL_TAG}/mithril-infra/configuration/${MITHRIL_NETWORK}"

GENESIS_VERIFICATION_KEY=$(curl -fsSL "${vkey_base}/genesis.vkey")
export GENESIS_VERIFICATION_KEY

ANCILLARY_VERIFICATION_KEY=$(curl -fsSL "${vkey_base}/ancillary.vkey")
export ANCILLARY_VERIFICATION_KEY

# The Cardano database v1 backend was dropped in Mithril distribution 2630.0, so
# we use the v2 backend (now the default). Ancillary files carry the ledger
# state that lets `cardano-node` start without re-syncing from genesis, so we
# include them; that in turn requires the ancillary verification key.
/app/bin/mithril-client cardano-db download latest \
  --include-ancillary \
  --download-dir "$UNPACK_DIR" \
  --json
