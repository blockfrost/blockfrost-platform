#!/usr/bin/env bash

set -euo pipefail

# ---------------------------------------------------------------------------- #

work_dir=""
child_pids=()
cleanup() {
  kill "${child_pids[@]}" 2>/dev/null || true
  wait
  if [ -n "$work_dir" ]; then
    cd /
    rm -rf -- "$work_dir"
  fi
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
    local color_yellow=$'\e[93m'
    local color_green=$'\e[92m'
    case "$level" in
    "FATAL") level="${color_red}${level}${color_reset}" ;;
    " WARN") level="${color_yellow}${level}${color_reset}" ;;
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
log info "Deriving keys from the ‘SUBMIT_MNEMONIC’"

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
    --shelley-payment-key --signing-key-file "payment.prv" --out-file "payment.sk"

  # Extract the payment verification key
  cardano-cli key verification-key --signing-key-file "payment.sk" \
    --verification-key-file "payment.evkey"

  # Convert the extended payment verification key to a non-extended key
  cardano-cli key non-extended-key \
    --extended-verification-key-file "payment.evkey" \
    --verification-key-file "payment.vk"

  # Convert stake signing key to cardano-cli format
  cardano-cli key convert-cardano-address-key \
    --shelley-stake-key --signing-key-file "stake.prv" --out-file "stake.sk"

  # Extract the stake verification key
  cardano-cli key verification-key --signing-key-file "stake.sk" \
    --verification-key-file "stake.evkey"

  # Convert the extended stake verification key to a non-extended key
  cardano-cli key non-extended-key \
    --extended-verification-key-file "stake.evkey" \
    --verification-key-file "stake.vk"

  # Generate base address using non-extended verification keys
  cardano-cli address build \
    --payment-verification-key-file "payment.vk" \
    --stake-verification-key-file "stake.vk" >"payment.addr"

  log info "‘SUBMIT_MNEMONIC’ address: $(cat "payment.addr")"
)

# ---------------------------------------------------------------------------- #

log info "Generating L1 credentials…"

for participant in alice-funds alice-node bob-funds bob-node; do
  log info "Generating L1 credentials for: $participant"

  mkdir -p credentials/"$participant"

  cardano-cli address key-gen \
    --verification-key-file credentials/"$participant"/payment.vk \
    --signing-key-file credentials/"$participant"/payment.sk

  cardano-cli address build \
    --verification-key-file credentials/"$participant"/payment.vk \
    --out-file credentials/"$participant"/payment.addr
done

# ---------------------------------------------------------------------------- #

log info "Funding L1 participants and nodes: alice, bob"

declare -A lovelace_fund
lovelace_fund["alice-funds"]=30000000
lovelace_fund["alice-node"]=30000000
lovelace_fund["bob-funds"]=30000000
lovelace_fund["bob-node"]=30000000

txdir=tx-01-fund-participants
mkdir -p $txdir

cardano-cli query utxo \
  --address "$(cat credentials/submit-mnemonic/payment.addr)" \
  --out-file $txdir/input-utxo.json

# shellcheck disable=SC2046
cardano-cli latest transaction build \
  $(jq <$txdir/input-utxo.json -j 'to_entries[].key | "--tx-in ", ., " "') \
  --change-address "$(cat credentials/submit-mnemonic/payment.addr)" \
  --tx-out "$(cat credentials/alice-funds/payment.addr)"+"${lovelace_fund["alice-funds"]}" \
  --tx-out "$(cat credentials/alice-node/payment.addr)"+"${lovelace_fund["alice-node"]}" \
  --tx-out "$(cat credentials/bob-funds/payment.addr)"+"${lovelace_fund["bob-funds"]}" \
  --tx-out "$(cat credentials/bob-node/payment.addr)"+"${lovelace_fund["bob-node"]}" \
  --out-file $txdir/tx.json

cardano-cli latest transaction sign \
  --tx-file $txdir/tx.json \
  --signing-key-file credentials/submit-mnemonic/payment.sk \
  --out-file $txdir/tx-signed.json

cardano-cli latest transaction submit --tx-file $txdir/tx-signed.json

# ---------------------------------------------------------------------------- #

for participant in alice-funds alice-node bob-funds bob-node; do
  while true; do
    funds=$(cardano-cli query utxo --address "$(cat credentials/"$participant"/payment.addr)" --out-file /dev/stdout | jq --compact-output .)
    log info "Verifying L1 participant funds: $participant: $funds"
    if [ "$funds" != '{}' ]; then
      break
    fi
    sleep 5
  done
done

# ---------------------------------------------------------------------------- #

log info "Waiting for Blockfrost to index the new addresses (so that the ‘--blockfrost’ Hydra sees it, too)"

for participant in alice-funds alice-node bob-funds bob-node; do
  while true; do
    resp="$(curl -sS -w $'\n%{http_code}' \
      -H "project_id: $BLOCKFROST_PROJECT_ID" \
      "https://cardano-${NETWORK}.blockfrost.io/api/v0/addresses/$(cat "credentials/$participant/payment.addr")/utxos?count=1")"
    body="${resp%$'\n'*}"
    code="${resp##*$'\n'}"
    log info "Verifying L1 participant funds with Blockfrost: $participant: http/$code…"
    if [ "$code" = "200" ] && jq -e 'type=="array" and length>0' >/dev/null <<<"$body"; then
      log info "… $(jq -c . <<<"$body")"
      break
    fi
    sleep 5
  done
done

# ---------------------------------------------------------------------------- #

for participant in alice bob; do
  log info "Generating L2 credentials for: $participant"

  hydra-node gen-hydra-key --output-file credentials/"$participant"-node/hydra
done

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

declare -A hydra_api_port
hydra_api_port["alice"]=$(python3 -m portpicker)
hydra_api_port["bob"]=$(python3 -m portpicker)

declare -A hydra_h2h_port
hydra_h2h_port["alice"]=$(python3 -m portpicker)
hydra_h2h_port["bob"]=$(python3 -m portpicker)

declare -A hydra_metrics_port
hydra_metrics_port["alice"]=$(python3 -m portpicker)
hydra_metrics_port["bob"]=$(python3 -m portpicker)

log info "Starting the Hydra node for: alice"

# Setting `CONTESTATION_PERIOD_SECONDS` to 10s makes the Head unclosable. I
# think because our Close tx isn’t making it on-chain. With 10s, the node builds
# a Close whose validity window ends at “now + 10s.” Cardano’s average block
# time is ~20s, so by the time a block comes, the tx is already outside its
# validity interval and gets dropped from the mempool—so the Head looks like it
# stays open.
#
# OTOH, the default makes us wait 10 minutes before Fanout.
#
# @michalrus tested 60s to be a good compromise for testnet tests.

export CONTESTATION_PERIOD_SECONDS=60

hydra-node \
  --node-id "alice-node" \
  --persistence-dir persistence-alice \
  --cardano-signing-key credentials/alice-node/payment.sk \
  --hydra-signing-key credentials/alice-node/hydra.sk \
  --hydra-scripts-tx-id "$HYDRA_SCRIPTS_TX_ID" \
  --ledger-protocol-parameters protocol-parameters.json \
  --contestation-period "$CONTESTATION_PERIOD_SECONDS"s \
  --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
  --node-socket "$CARDANO_NODE_SOCKET_PATH" \
  --api-port "${hydra_api_port["alice"]}" \
  --api-host 127.0.0.1 \
  --listen 127.0.0.1:"${hydra_h2h_port["alice"]}" \
  --peer 127.0.0.1:"${hydra_h2h_port["bob"]}" \
  --monitoring-port "${hydra_metrics_port["alice"]}" \
  --hydra-verification-key credentials/bob-node/hydra.vk \
  --cardano-verification-key credentials/bob-node/payment.vk \
  &
child_pids+=($!)

# ---------------------------------------------------------------------------- #

log info "Starting the Hydra node for: bob"

hydra-node \
  --node-id "bob-node" \
  --persistence-dir persistence-bob \
  --cardano-signing-key credentials/bob-node/payment.sk \
  --hydra-signing-key credentials/bob-node/hydra.sk \
  --hydra-scripts-tx-id "$HYDRA_SCRIPTS_TX_ID" \
  --ledger-protocol-parameters protocol-parameters.json \
  --contestation-period "$CONTESTATION_PERIOD_SECONDS"s \
  --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
  --node-socket "$CARDANO_NODE_SOCKET_PATH" \
  --api-port "${hydra_api_port["bob"]}" \
  --listen 127.0.0.1:"${hydra_h2h_port["bob"]}" \
  --api-host 127.0.0.1 \
  --peer 127.0.0.1:"${hydra_h2h_port["alice"]}" \
  --monitoring-port "${hydra_metrics_port["bob"]}" \
  --hydra-verification-key credentials/alice-node/hydra.vk \
  --cardano-verification-key credentials/alice-node/payment.vk \
  &
child_pids+=($!)

# ---------------------------------------------------------------------------- #

while true; do
  sleep 1
  log info "Waiting for ‘hydra-node’s to connect…"
  num_peers=$(curl -fsSL http://127.0.0.1:"${hydra_metrics_port["bob"]}"/metrics | grep ^hydra_head_peers_connected | awk '{print $2}')
  if [ "$num_peers" == "1.0" ]; then
    log info "Number of peers: $num_peers"
    break
  fi
done

# ---------------------------------------------------------------------------- #

log info "Opening a Hydra head"

{
  echo '{"tag":"Init"}'
  sleep 2 # This one works with `--one-message` and without `sleep`, but other calls don’t, so just in case.
} | websocat ws://127.0.0.1:"${hydra_api_port["alice"]}"/

while true; do
  sleep 3
  status=$(curl -fsSL http://127.0.0.1:"${hydra_api_port["alice"]}"/head | jq -r .tag)
  log info "Waiting for ‘Initial’; head status: $status"
  if [ "$status" == "Initial" ]; then
    break
  fi
done

# ---------------------------------------------------------------------------- #

log info "Committing L1 funds to the head…"

txdir=tx-02-commit-L1-to-L2
mkdir -p $txdir

for participant in alice bob; do
  log info "Committing L1 funds to the head: $participant"

  cardano-cli query utxo \
    --address "$(cat credentials/"$participant"-funds/payment.addr)" \
    --out-file $txdir/commit-utxo-"$participant".json

  curl -fsSL -X POST http://127.0.0.1:"${hydra_api_port[$participant]}"/commit \
    --data @$txdir/commit-utxo-"$participant".json \
    >$txdir/commit-tx-"$participant".json

  cardano-cli latest transaction sign \
    --tx-file $txdir/commit-tx-"$participant".json \
    --signing-key-file credentials/"$participant"-funds/payment.sk \
    --out-file $txdir/commit-tx-signed-"$participant".json

  cardano-cli latest transaction submit --tx-file $txdir/commit-tx-signed-"$participant".json
done

# ---------------------------------------------------------------------------- #

while true; do
  sleep 3
  status=$(curl -fsSL http://127.0.0.1:"${hydra_api_port["alice"]}"/head | jq -r .tag)
  log info "Waiting for ‘Committed’ and ‘HeadIsOpen’; head status: $status"
  if [ "$status" == "Open" ]; then
    break
  fi
done

# ---------------------------------------------------------------------------- #

log info "Using the Hydra head"

txdir=tx-03-on-L2
mkdir -p $txdir

curl -fsSL http://127.0.0.1:"${hydra_api_port["alice"]}"/snapshot/utxo |
  jq "with_entries(select(.value.address == \"$(cat credentials/alice-funds/payment.addr)\"))" \
    >$txdir/utxo.json

lovelace_L2=1000000
cardano-cli latest transaction build-raw \
  --tx-in "$(jq <$txdir/utxo.json -r 'to_entries[0].key')" \
  --tx-out "$(cat credentials/bob-funds/payment.addr)"+"${lovelace_L2}" \
  --tx-out "$(cat credentials/alice-funds/payment.addr)"+"$(jq <$txdir/utxo.json "to_entries[0].value.value.lovelace - ${lovelace_L2}")" \
  --fee 0 \
  --out-file $txdir/tx.json

cardano-cli latest transaction sign \
  --tx-body-file $txdir/tx.json \
  --signing-key-file credentials/alice-funds/payment.sk \
  --out-file $txdir/tx-signed.json

{
  jq <$txdir/tx-signed.json -c '{tag: "NewTx", transaction: .}'
  sleep 2 # This one works with `--one-message` and without `sleep`, but other calls don’t, so just in case.
} | websocat ws://127.0.0.1:"${hydra_api_port["alice"]}"/

# ---------------------------------------------------------------------------- #

log info "Verifying snapshot UTxO"

while true; do
  sleep 3
  count=$(curl -fsSL http://127.0.0.1:"${hydra_api_port["alice"]}"/snapshot/utxo | jq 'length')
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
} | websocat ws://127.0.0.1:"${hydra_api_port["alice"]}"/

while true; do
  sleep 3
  status=$(curl -fsSL http://127.0.0.1:"${hydra_api_port["alice"]}"/head | jq -r .tag)
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
} | websocat ws://127.0.0.1:"${hydra_api_port["alice"]}"/

while true; do
  sleep 3
  status=$(curl -fsSL http://127.0.0.1:"${hydra_api_port["alice"]}"/head | jq -r .tag)
  log info "Waiting for ‘Idle’; head status: $status"
  if [ "$status" == "Idle" ]; then
    break
  fi
done

# ---------------------------------------------------------------------------- #

log info "Verifying that the funds were moved from Alice on L1…"

new_alice_funds=$(cardano-cli query utxo \
  --address "$(cat credentials/alice-funds/payment.addr)" \
  --out-file /dev/stdout |
  jq '[.[] | .value.lovelace] | add // 0')
new_bob_funds=$(cardano-cli query utxo \
  --address "$(cat credentials/bob-funds/payment.addr)" \
  --out-file /dev/stdout |
  jq '[.[] | .value.lovelace] | add // 0')

if ((new_alice_funds == "${lovelace_fund["alice-funds"]}" - lovelace_L2)) &&
  ((new_bob_funds == "${lovelace_fund["bob-funds"]}" + lovelace_L2)); then
  log info "… OK, funds were moved correctly on L1."
else
  log fatal "Unexpected total UTxO, Alice: $new_alice_funds, Bob: $new_bob_funds."
  exit 1
fi

unset new_alice_funds
unset new_bob_funds

# ---------------------------------------------------------------------------- #

log info "Returning all funds to ‘SUBMIT_MNEMONIC’…"

txdir=tx-04-return-test-ada
mkdir -p $txdir

declare -A lovelace_remaining

for participant in alice-funds bob-funds alice-node bob-node; do
  log info "Returning funds from $participant to ‘SUBMIT_MNEMONIC’"

  cardano-cli query utxo \
    --address "$(cat credentials/"$participant"/payment.addr)" \
    --out-file $txdir/utxo-"$participant".json

  lovelace_remaining["$participant"]=$(jq '[.[] | .value.lovelace] | add // 0' $txdir/utxo-"$participant".json)

  # shellcheck disable=SC2046
  cardano-cli latest transaction build \
    $(jq <$txdir/utxo-"$participant".json -j 'to_entries[].key | "--tx-in ", ., " "') \
    --change-address "$(cat credentials/submit-mnemonic/payment.addr)" \
    --out-file $txdir/tx-"$participant".json

  cardano-cli latest transaction sign \
    --tx-file $txdir/tx-"$participant".json \
    --signing-key-file credentials/"$participant"/payment.sk \
    --out-file $txdir/tx-signed-"$participant".json

  cardano-cli latest transaction submit --tx-file $txdir/tx-signed-"$participant".json
done

# ---------------------------------------------------------------------------- #

log info "Calculating how much was lost in Hydra transaction fees (excluding L1 fees from and to ‘SUBMIT_MNEMONIC’)…"

lovelace_to_ada() {
  printf '%d.%06d' $(($1 / 1000000)) $(($1 % 1000000))
}

total_cost=0

for participant in alice-funds bob-funds alice-node bob-node; do
  cost=$((lovelace_fund["$participant"] - lovelace_remaining["$participant"]))
  total_cost=$((total_cost + cost))
  log info "Address ‘$participant’ lost $(lovelace_to_ada "$cost") ADA."
done

log warn "In total, we lost $(lovelace_to_ada "$total_cost") ADA (probably in Hydra transaction fees)."

# ---------------------------------------------------------------------------- #

log info "Exiting."
