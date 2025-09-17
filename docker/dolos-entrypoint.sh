#!/bin/sh

set -eu

case "$NETWORK" in
mainnet)
  export PEER_ADDRESS="backbone.cardano.iog.io:3001"
  export NETWORK_MAGIC="764824073"
  export IS_TESTNET="false"
  export MITHRIL_AGGREGATOR="https://aggregator.release-mainnet.api.mithril.network/aggregator"
  export MITHRIL_GENESIS_KEY="5b3139312c36362c3134302c3138352c3133382c31312c3233372c3230372c3235302c3134342c32372c322c3138382c33302c31322c38312c3135352c3230342c31302c3137392c37352c32332c3133382c3139362c3231372c352c31342c32302c35372c37392c33392c3137365d"
  ;;
preprod)
  export PEER_ADDRESS="preprod-node.play.dev.cardano.org:3001"
  export NETWORK_MAGIC="1"
  export IS_TESTNET="true"
  export MITHRIL_AGGREGATOR="https://aggregator.release-preprod.api.mithril.network/aggregator"
  export MITHRIL_GENESIS_KEY="5b3132372c37332c3132342c3136312c362c3133372c3133312c3231332c3230372c3131372c3139382c38352c3137362c3139392c3136322c3234312c36382c3132332c3131392c3134352c31332c3233322c3234332c34392c3232392c322c3234392c3230352c3230352c33392c3233352c34345d"
  ;;
preview)
  export PEER_ADDRESS="preview-node.play.dev.cardano.org:3001"
  export NETWORK_MAGIC="2"
  export IS_TESTNET="true"
  export MITHRIL_AGGREGATOR="https://aggregator.pre-release-preview.api.mithril.network/aggregator"
  export MITHRIL_GENESIS_KEY="5b3132372c37332c3132342c3136312c362c3133372c3133312c3231332c3230372c3131372c3139382c38352c3137362c3139392c3136322c3234312c36382c3132332c3131392c3134352c31332c3233322c3234332c34392c3232392c322c3234392c3230352c3230352c33392c3233352c34345d"
  ;;
*)
  echo >&2 "fatal: Unsupported NETWORK='$NETWORK'. Expected: mainnet, preprod, preview."
  exit 1
  ;;
esac

cat </config.toml.tpl |
  sed -re 's#NETWORK_MAGIC#'"$NETWORK_MAGIC"'#g' |
  sed -re 's#NETWORK#'"$NETWORK"'#g' |
  sed -re 's#PEER_ADDRESS#'"$PEER_ADDRESS"'#g' |
  sed -re 's#IS_TESTNET#'"$IS_TESTNET"'#g' |
  sed -re 's#MITHRIL_AGGREGATOR#'"$MITHRIL_AGGREGATOR"'#g' |
  sed -re 's#MITHRIL_GENESIS_KEY#'"$MITHRIL_GENESIS_KEY"'#g' \
    >/config.toml

cd /data/

# We bootstrap from Mithril, because it’s safer.
# `/data/snapshot` will be cleared once the bootstrap process finishes.
test -e /data/chain || dolos --config /config.toml bootstrap mithril --download-dir /data/snapshot

exec dolos --config /config.toml daemon
