#!/bin/sh
#
# Generate a Dolos TOML config for a given network.
#
# Usage: generate-dolos-config.sh [OPTIONS] NETWORK
#
# Options:
#   --genesis-prefix DIR   Directory prefix for genesis files (default: /etc/genesis)
#   --storage-path PATH    Storage path value (default: /data)
#
# The generated config is written to stdout.

set -eu

genesis_prefix="/etc/genesis"
storage_path="/data"

while [ $# -gt 0 ]; do
  case "$1" in
  --genesis-prefix)
    genesis_prefix="$2"
    shift 2
    ;;
  --storage-path)
    storage_path="$2"
    shift 2
    ;;
  -*)
    echo >&2 "fatal: Unknown option: $1"
    exit 1
    ;;
  *)
    break
    ;;
  esac
done

NETWORK="${1:?fatal: NETWORK argument required (mainnet, preprod, preview)}"

case "$NETWORK" in
mainnet)
  PEER_ADDRESS="backbone.cardano.iog.io:3001"
  NETWORK_MAGIC="764824073"
  IS_TESTNET="false"
  MITHRIL_AGGREGATOR="https://aggregator.release-mainnet.api.mithril.network/aggregator"
  MITHRIL_ANCILLARY_KEY="5b32332c37312c39362c3133332c34372c3235332c3232362c3133362c3233352c35372c3136342c3130362c3138362c322c32312c32392c3132302c3136332c38392c3132312c3137372c3133382c3230382c3133382c3231342c39392c35382c32322c302c35382c332c36395d"
  MITHRIL_GENESIS_KEY="5b3139312c36362c3134302c3138352c3133382c31312c3233372c3230372c3235302c3134342c32372c322c3138382c33302c31322c38312c3135352c3230342c31302c3137392c37352c32332c3133382c3139362c3231372c352c31342c32302c35372c37392c33392c3137365d"
  TOKEN_REGISTRY_URL="https://tokens.cardano.org"
  FORCE_PROTOCOL_LINE=""
  ;;
preprod)
  PEER_ADDRESS="preprod-node.play.dev.cardano.org:3001"
  NETWORK_MAGIC="1"
  IS_TESTNET="true"
  MITHRIL_AGGREGATOR="https://aggregator.release-preprod.api.mithril.network/aggregator"
  MITHRIL_ANCILLARY_KEY="5b3138392c3139322c3231362c3135302c3131342c3231362c3233372c3231302c34352c31382c32312c3139362c3230382c3234362c3134362c322c3235322c3234332c3235312c3139372c32382c3135372c3230342c3134352c33302c31342c3232382c3136382c3132392c38332c3133362c33365d"
  MITHRIL_GENESIS_KEY="5b3132372c37332c3132342c3136312c362c3133372c3133312c3231332c3230372c3131372c3139382c38352c3137362c3139392c3136322c3234312c36382c3132332c3131392c3134352c31332c3233322c3234332c34392c3232392c322c3234392c3230352c3230352c33392c3233352c34345d"
  TOKEN_REGISTRY_URL="https://metadata.world.dev.cardano.org"
  FORCE_PROTOCOL_LINE=""
  ;;
preview)
  PEER_ADDRESS="preview-node.play.dev.cardano.org:3001"
  NETWORK_MAGIC="2"
  IS_TESTNET="true"
  MITHRIL_AGGREGATOR="https://aggregator.pre-release-preview.api.mithril.network/aggregator"
  MITHRIL_ANCILLARY_KEY="5b3138392c3139322c3231362c3135302c3131342c3231362c3233372c3231302c34352c31382c32312c3139362c3230382c3234362c3134362c322c3235322c3234332c3235312c3139372c32382c3135372c3230342c3134352c33302c31342c3232382c3136382c3132392c38332c3133362c33365d"
  MITHRIL_GENESIS_KEY="5b3132372c37332c3132342c3136312c362c3133372c3133312c3231332c3230372c3131372c3139382c38352c3137362c3139392c3136322c3234312c36382c3132332c3131392c3134352c31332c3233322c3234332c34392c3232392c322c3234392c3230352c3230352c33392c3233352c34345d"
  TOKEN_REGISTRY_URL="https://metadata.world.dev.cardano.org"
  FORCE_PROTOCOL_LINE="force_protocol = 6"
  ;;
*)
  echo >&2 "fatal: Unsupported NETWORK='$NETWORK'. Expected: mainnet, preprod, preview."
  exit 1
  ;;
esac

SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"

sed -re 's#NETWORK_MAGIC#'"$NETWORK_MAGIC"'#g' "$SCRIPT_DIR/dolos-config.toml.tpl" |
  sed -re 's#NETWORK#'"$NETWORK"'#g' |
  sed -re 's#PEER_ADDRESS#'"$PEER_ADDRESS"'#g' |
  sed -re 's#IS_TESTNET#'"$IS_TESTNET"'#g' |
  sed -re 's#MITHRIL_AGGREGATOR#'"$MITHRIL_AGGREGATOR"'#g' |
  sed -re 's#MITHRIL_ANCILLARY_KEY#'"$MITHRIL_ANCILLARY_KEY"'#g' |
  sed -re 's#MITHRIL_GENESIS_KEY#'"$MITHRIL_GENESIS_KEY"'#g' |
  sed -re 's#TOKEN_REGISTRY_URL#'"$TOKEN_REGISTRY_URL"'#g' |
  sed -re 's#STORAGE_PATH#'"$storage_path"'#g' |
  sed -re 's#GENESIS_PREFIX#'"$genesis_prefix"'#g' |
  if [ -n "$FORCE_PROTOCOL_LINE" ]; then
    sed -re 's#FORCE_PROTOCOL_LINE#'"$FORCE_PROTOCOL_LINE"'#g'
  else
    sed -re '/FORCE_PROTOCOL_LINE/d'
  fi
