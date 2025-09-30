#!/usr/bin/env bash

set -euo pipefail

# ---------------------------------------------------------------------------- #

child_pids=()
cleanup() {
  kill "${child_pids[@]}" 2>/dev/null || true
  wait
}
trap cleanup INT TERM EXIT

# ---------------------------------------------------------------------------- #

log() {
  local level="${1}"
  shift
  level=$(printf '%5s' "${level^^}")
  local timestamp
  timestamp=$(date -u +'%Y-%m-%dT%H:%M:%S.%6NZ')
  if [[ -t 2 ]]; then
    local color_reset=$'\e[0m'
    local color_grey=$'\e[90m'
    local color_red=$'\e[1;91m'
    local color_green=$'\e[92m'
    case "$level" in
    "FATAL") level="${color_red}${level}${color_reset}" ;;
    " INFO") level="${color_green}${level}${color_reset}" ;;
    esac
    timestamp="${color_grey}${timestamp}${color_reset}"
  fi
  echo >&2 "$timestamp" "$level" "$@"
}

require_env() {
  local name="$1"
  local val="${!name-}"
  if [[ -z $val ]]; then
    log fatal "$name is not set."
    missing=1
  fi
}
missing=0
for v in NETWORK BLOCKFROST_PROJECT_ID CARDANO_NODE_SOCKET_PATH SUBMIT_MNEMONIC CARDANO_NODE_NETWORK_ID HYDRA_SCRIPTS_TX_ID; do
  require_env "$v"
done
if ((missing)); then
  exit 1
fi

# ---------------------------------------------------------------------------- #

work_dir=$(mktemp -d)
cd "$work_dir"

log info "Working directory: $work_dir"
log info "Network tip: $(cardano-cli query tip | jq --compact-output .)"

mkdir -p credentials

# ---------------------------------------------------------------------------- #

# Derive keys using cardano-address
log info "Deriving keys from the SUBMIT_MNEMONIC"

(
  mkdir -p credentials/submit-mnemonic
  cd credentials/submit-mnemonic

  echo "$SUBMIT_MNEMONIC" | cardano-address key from-recovery-phrase Shelley >"root.prv"

  # Derive payment key (m/1852'/1815'/0'/0/0)
  cardano-address key child 1852H/1815H/0H/0/0 <"root.prv" >"payment.prv"
  cardano-address key public --with-chain-code <"payment.prv" >"payment.pub"

  # Derive stake key (m/1852'/1815'/0'/2/0)
  cardano-address key child 1852H/1815H/0H/2/0 <"root.prv" >"stake.prv"
  cardano-address key public --with-chain-code <"stake.prv" >"stake.pub"

  # Convert payment signing key to cardano-cli format
  cardano-cli key convert-cardano-address-key \
    --shelley-payment-key --signing-key-file "payment.prv" --out-file "payment.skey"

  # Extract the payment verification key
  cardano-cli key verification-key --signing-key-file "payment.skey" \
    --verification-key-file "payment.evkey"

  # Convert the extended payment verification key to a non-extended key
  cardano-cli key non-extended-key \
    --extended-verification-key-file "payment.evkey" \
    --verification-key-file "payment.vkey"

  # Convert stake signing key to cardano-cli format
  cardano-cli key convert-cardano-address-key \
    --shelley-stake-key --signing-key-file "stake.prv" --out-file "stake.skey"

  # Extract the stake verification key
  cardano-cli key verification-key --signing-key-file "stake.skey" \
    --verification-key-file "stake.evkey"

  # Convert the extended stake verification key to a non-extended key
  cardano-cli key non-extended-key \
    --extended-verification-key-file "stake.evkey" \
    --verification-key-file "stake.vkey"

  # Generate base address using non-extended verification keys
  cardano-cli address build \
    --payment-verification-key-file "payment.vkey" \
    --stake-verification-key-file "stake.vkey" >"payment.addr"

  log info "SUBMIT_MNEMONIC address: $(cat "payment.addr")"
)

# ---------------------------------------------------------------------------- #

log info "Generating L1 credentials for: alice"

cardano-cli address key-gen \
  --verification-key-file credentials/alice-node.vk \
  --signing-key-file credentials/alice-node.sk

cardano-cli address build \
  --verification-key-file credentials/alice-node.vk \
  --out-file credentials/alice-node.addr

cardano-cli address key-gen \
  --verification-key-file credentials/alice-funds.vk \
  --signing-key-file credentials/alice-funds.sk

cardano-cli address build \
  --verification-key-file credentials/alice-funds.vk \
  --out-file credentials/alice-funds.addr

# ---------------------------------------------------------------------------- #

log info "Generating L1 credentials for: bob"

cardano-cli address key-gen \
  --verification-key-file credentials/bob-node.vk \
  --signing-key-file credentials/bob-node.sk

cardano-cli address build \
  --verification-key-file credentials/bob-node.vk \
  --out-file credentials/bob-node.addr

cardano-cli address key-gen \
  --verification-key-file credentials/bob-funds.vk \
  --signing-key-file credentials/bob-funds.sk

cardano-cli address build \
  --verification-key-file credentials/bob-funds.vk \
  --out-file credentials/bob-funds.addr

# ---------------------------------------------------------------------------- #

log info "Funding L1 participants: alice, bob"

cardano-cli query utxo \
  --address "$(cat credentials/submit-mnemonic/payment.addr)" \
  --out-file submit-mnemonic-funds-utxo.json

# shellcheck disable=SC2046
cardano-cli latest transaction build \
  $(jq <submit-mnemonic-funds-utxo.json -j 'to_entries[].key | "--tx-in ", ., " "') \
  --change-address "$(cat credentials/submit-mnemonic/payment.addr)" \
  --tx-out "$(cat credentials/alice-funds.addr)"+30000000 \
  --tx-out "$(cat credentials/alice-node.addr)"+30000000 \
  --tx-out "$(cat credentials/bob-funds.addr)"+30000000 \
  --tx-out "$(cat credentials/bob-node.addr)"+30000000 \
  --out-file tx.json

cardano-cli latest transaction sign \
  --tx-file tx.json \
  --signing-key-file credentials/submit-mnemonic/payment.skey \
  --out-file tx-signed.json

cardano-cli latest transaction submit --tx-file tx-signed.json

# ---------------------------------------------------------------------------- #

for who in alice-funds alice-node bob-funds bob-node; do
  while true; do
    funds=$(cardano-cli query utxo --address "$(cat credentials/"$who".addr)" --out-file /dev/stdout | jq --compact-output .)
    log info "Verifying L1 participant funds: $who: $funds"
    if [ "$funds" != '{}' ]; then
      break
    fi
    sleep 5
  done
done

# ---------------------------------------------------------------------------- #

log info "Generating L2 credentials for: alice"

hydra-node gen-hydra-key --output-file credentials/alice-hydra

# ---------------------------------------------------------------------------- #

log info "Generating L2 credentials for: bob"

hydra-node gen-hydra-key --output-file credentials/bob-hydra

# ---------------------------------------------------------------------------- #

log info "Eliminating transaction fees from L2 protocol parameters"

cardano-cli query protocol-parameters |
  jq '.
        | .txFeeFixed = 0
        | .txFeePerByte = 0
        | .executionUnitPrices.priceMemory = 0
        | .executionUnitPrices.priceSteps = 0
       ' \
    >protocol-parameters.json

# ---------------------------------------------------------------------------- #

log info "Starting the Hydra node for: alice"

# FIXME: Eh, setting it to 10s makes the Head unclosable. Let’s try 30 s.
#
# FIXME: The default makes us wait 10 minutes before Fanout — but it works eventually.
#
# FIXME: I think because our Close tx isn’t making it on-chain. With
# --contestation-period 10s the node builds a Close whose validity window
# basically ends at “now + 10s.” Cardano’s average block time is ~20s, so by the
# time a block comes, the tx is already outside its validity interval and gets
# dropped from the mempool—so the Head looks like it stays open.

export CONTESTATION_PERIOD_SECONDS=30

hydra-node \
  --node-id "alice-node" \
  --persistence-dir persistence-alice \
  --cardano-signing-key credentials/alice-node.sk \
  --hydra-signing-key credentials/alice-hydra.sk \
  --hydra-scripts-tx-id "$HYDRA_SCRIPTS_TX_ID" \
  --ledger-protocol-parameters protocol-parameters.json \
  --contestation-period "$CONTESTATION_PERIOD_SECONDS"s \
  --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
  --node-socket "$CARDANO_NODE_SOCKET_PATH" \
  --api-port 4001 \
  --api-host 127.0.0.1 \
  --listen 127.0.0.1:5001 \
  --peer 127.0.0.1:5002 \
  --monitoring-port 6001 \
  --hydra-verification-key credentials/bob-hydra.vk \
  --cardano-verification-key credentials/bob-node.vk \
  &
child_pids+=($!)

# ---------------------------------------------------------------------------- #

log info "Starting the Hydra node for: bob"

hydra-node \
  --node-id "bob-node" \
  --persistence-dir persistence-bob \
  --cardano-signing-key credentials/bob-node.sk \
  --hydra-signing-key credentials/bob-hydra.sk \
  --hydra-scripts-tx-id "$HYDRA_SCRIPTS_TX_ID" \
  --ledger-protocol-parameters protocol-parameters.json \
  --contestation-period "$CONTESTATION_PERIOD_SECONDS"s \
  --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
  --node-socket "$CARDANO_NODE_SOCKET_PATH" \
  --api-port 4002 \
  --listen 127.0.0.1:5002 \
  --api-host 127.0.0.1 \
  --peer 127.0.0.1:5001 \
  --monitoring-port 6002 \
  --hydra-verification-key credentials/alice-hydra.vk \
  --cardano-verification-key credentials/alice-node.vk \
  &
child_pids+=($!)

# ---------------------------------------------------------------------------- #

while true; do
  sleep 1
  log info "Waiting for ‘hydra-node’s to connect…"
  num_peers=$(curl -fsSL http://127.0.0.1:6002/metrics | grep ^hydra_head_peers_connected | awk '{print $2}')
  if [ "$num_peers" == "1.0" ]; then
    log info "Number of peers: $num_peers"
    break
  fi
done

# ---------------------------------------------------------------------------- #

log info "Opening a Hydra head"

echo '{"tag":"Init"}' | websocat --one-message ws://127.0.0.1:4001/

while true; do
  sleep 3
  status=$(curl -fsSL http://127.0.0.1:4001/head | jq -r .tag)
  log info "Waiting for ‘Initial’; head status: $status"
  if [ "$status" == "Initial" ]; then
    break
  fi
done

# ---------------------------------------------------------------------------- #

log info "Committing funds to the head: alice"

cardano-cli query utxo \
  --address "$(cat credentials/alice-funds.addr)" \
  --out-file alice-commit-utxo.json

curl -fsSL -X POST http://127.0.0.1:4001/commit \
  --data @alice-commit-utxo.json \
  >alice-commit-tx.json

cardano-cli latest transaction sign \
  --tx-file alice-commit-tx.json \
  --signing-key-file credentials/alice-funds.sk \
  --out-file alice-commit-tx-signed.json

cardano-cli latest transaction submit --tx-file alice-commit-tx-signed.json

# ---------------------------------------------------------------------------- #

log info "Committing funds to the head: bob"

cardano-cli query utxo \
  --address "$(cat credentials/bob-funds.addr)" \
  --out-file bob-commit-utxo.json

curl -fsSL -X POST http://127.0.0.1:4002/commit \
  --data @bob-commit-utxo.json \
  >bob-commit-tx.json

cardano-cli latest transaction sign \
  --tx-file bob-commit-tx.json \
  --signing-key-file credentials/bob-funds.sk \
  --out-file bob-commit-tx-signed.json

cardano-cli latest transaction submit --tx-file bob-commit-tx-signed.json

# ---------------------------------------------------------------------------- #

while true; do
  sleep 3
  status=$(curl -fsSL http://127.0.0.1:4001/head | jq -r .tag)
  log info "Waiting for ‘Committed’ and ‘HeadIsOpen’; head status: $status"
  if [ "$status" == "Open" ]; then
    break
  fi
done

# ---------------------------------------------------------------------------- #

log info "Using the Hydra head"

rm utxo.json tx.json tx-signed.json || true

curl -fsSL http://127.0.0.1:4001/snapshot/utxo |
  jq "with_entries(select(.value.address == \"$(cat credentials/alice-funds.addr)\"))" \
    >utxo.json

lovelace=1000000
cardano-cli latest transaction build-raw \
  --tx-in "$(jq <utxo.json -r 'to_entries[0].key')" \
  --tx-out "$(cat credentials/bob-funds.addr)"+"${lovelace}" \
  --tx-out "$(cat credentials/alice-funds.addr)"+"$(jq <utxo.json "to_entries[0].value.value.lovelace - ${lovelace}")" \
  --fee 0 \
  --out-file tx.json

cardano-cli latest transaction sign \
  --tx-body-file tx.json \
  --signing-key-file credentials/alice-funds.sk \
  --out-file tx-signed.json

jq <tx-signed.json -c '{tag: "NewTx", transaction: .}' | websocat --one-message ws://127.0.0.1:4001/

# ---------------------------------------------------------------------------- #

log info "Verifying snapshot UTxO"

while true; do
  sleep 3
  count=$(curl -fsSL http://127.0.0.1:4001/snapshot/utxo | jq 'length')
  log info "UTxO count: $count"
  if [ "$count" == "3" ]; then
    break
  fi
done

# ---------------------------------------------------------------------------- #

log info "Closing the Hydra head"

{
  echo '{"tag":"Close"}'
  sleep 2 # Otherwise: `Warp: Client closed connection prematurely`.
} | websocat ws://127.0.0.1:4001/

while true; do
  sleep 3
  status=$(curl -fsSL http://127.0.0.1:4001/head | jq -r .tag)
  log info "Waiting for ‘Closed’; head status: $status"
  if [ "$status" == "Closed" ]; then
    break
  fi
done

# ---------------------------------------------------------------------------- #

invalidity_period=$(((2 + 1) * CONTESTATION_PERIOD_SECONDS))

log info "Waiting ${invalidity_period}s for validity period before fan-out"

sleep "$invalidity_period"

# ---------------------------------------------------------------------------- #

log info "Requesting fan-out"

{
  echo '{"tag":"Fanout"}'
  sleep 2 # Otherwise: `Warp: Client closed connection prematurely`.
} | websocat ws://127.0.0.1:4001/

# ---------------------------------------------------------------------------- #

# if true ; then
#     exit 77
# fi

# ---------------------------------------------------------------------------- #

log info "Sleeping for investigation…"
sleep 1200

# ---------------------------------------------------------------------------- #

log info "TODO: Verifying that the funds were moved on L1"

# ---------------------------------------------------------------------------- #

log info "TODO: Returning all funds to SUBMIT_MNEMONIC"

# ---------------------------------------------------------------------------- #

log info "Exiting…"
