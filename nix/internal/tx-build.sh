#!/usr/bin/env bash

set -euo pipefail

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

if [ "$#" -ne 3 ]; then
  log fatal "Usage: tx-build <NETWORK> <AMOUNT> <RECEIVING_ADDRESS>"
  log info
  log info "<NETWORK> is one of: preview, preprod, mainnet"
  log info
  log info "<AMOUNT> is in ADA (floating point)"
  log info
  log info "Also set your wallet’s RECOVERY_PHRASE environment variable,"
  log info "e.g. add it to ‘$PRJ_ROOT/.envrc.local’."
  log info
  log info "Debug messages will be written to stderr, while the CBOR of the transaction to stdout."
  exit 1
fi

NETWORK="$1"
AMOUNT="$2"
RECEIVING_ADDRESS="$3"

case "$NETWORK" in
mainnet)
  export CARDANO_NODE_NETWORK_ID="mainnet"
  ;;
preprod)
  export CARDANO_NODE_NETWORK_ID="1"
  ;;
preview)
  export CARDANO_NODE_NETWORK_ID="2"
  ;;
*)
  log fatal "invalid network, choose one from: mainnet, preprod, preview"
  exit 1
  ;;
esac

if [ -z "${CARDANO_NODE_SOCKET_PATH:-}" ]; then
  log fatal "CARDANO_NODE_SOCKET_PATH environment variable not set"
  exit 1
fi

if [ -z "${RECOVERY_PHRASE:-}" ]; then
  log fatal "RECOVERY_PHRASE environment variable not set"
  log info "you can add it to ‘$PRJ_ROOT/.envrc.local’"
  exit 1
fi

log info "network: $NETWORK"

if ! cardano-cli address info --address "$RECEIVING_ADDRESS" >/dev/null 2>&1; then
  log fatal "invalid receiving address: $RECEIVING_ADDRESS"
  exit 1
fi

log info "receiving address: $RECEIVING_ADDRESS"
log info "amount: $AMOUNT ADA"

TEMP_DIR=$(mktemp -d)
log info "using a temporary directory: $TEMP_DIR"

# shellcheck disable=SC2064
trap "rm -rf \"$TEMP_DIR\"" EXIT

# Derive keys using cardano-address
log info "deriving keys from the recovery phrase"
echo "$RECOVERY_PHRASE" | cardano-address key from-recovery-phrase Shelley >"$TEMP_DIR/root.prv"

# Derive payment key (m/1852'/1815'/0'/0/0)
cardano-address key child 1852H/1815H/0H/0/0 <"$TEMP_DIR/root.prv" >"$TEMP_DIR/payment.prv"
cardano-address key public --with-chain-code <"$TEMP_DIR/payment.prv" >"$TEMP_DIR/payment.pub"

# Derive stake key (m/1852'/1815'/0'/2/0)
cardano-address key child 1852H/1815H/0H/2/0 <"$TEMP_DIR/root.prv" >"$TEMP_DIR/stake.prv"
cardano-address key public --with-chain-code <"$TEMP_DIR/stake.prv" >"$TEMP_DIR/stake.pub"

# Convert payment signing key to cardano-cli format
cardano-cli key convert-cardano-address-key \
  --shelley-payment-key --signing-key-file "$TEMP_DIR/payment.prv" --out-file "$TEMP_DIR/payment.skey"

# Extract the payment verification key
cardano-cli key verification-key --signing-key-file "$TEMP_DIR/payment.skey" \
  --verification-key-file "$TEMP_DIR/payment.evkey"

# Convert the extended payment verification key to a non-extended key
cardano-cli key non-extended-key \
  --extended-verification-key-file "$TEMP_DIR/payment.evkey" \
  --verification-key-file "$TEMP_DIR/payment.vkey"

# Convert stake signing key to cardano-cli format
cardano-cli key convert-cardano-address-key \
  --shelley-stake-key --signing-key-file "$TEMP_DIR/stake.prv" --out-file "$TEMP_DIR/stake.skey"

# Extract the stake verification key
cardano-cli key verification-key --signing-key-file "$TEMP_DIR/stake.skey" \
  --verification-key-file "$TEMP_DIR/stake.evkey"

# Convert the extended stake verification key to a non-extended key
cardano-cli key non-extended-key \
  --extended-verification-key-file "$TEMP_DIR/stake.evkey" \
  --verification-key-file "$TEMP_DIR/stake.vkey"

# Generate base address using non-extended verification keys
MY_ADDRESS=$(cardano-cli address build \
  --payment-verification-key-file "$TEMP_DIR/payment.vkey" \
  --stake-verification-key-file "$TEMP_DIR/stake.vkey")
echo "$MY_ADDRESS" >"$TEMP_DIR/payment.addr"
log info "your address: $MY_ADDRESS"

# Query UTxOs
log info "querying UTxOs…"
cardano-cli query utxo --address "$MY_ADDRESS" --out-file "$TEMP_DIR/utxos.json"

UTXO_COUNT=$(jq length "$TEMP_DIR/utxos.json")
if [ "$UTXO_COUNT" -eq 0 ]; then
  log fatal "no UTxOs found at address $MY_ADDRESS"
  exit 1
fi
log info "UTxO count: $UTXO_COUNT"

# Build raw transaction
log info "building transaction…"

TX_INS=()
TOTAL_LOVELACE=0

for UTXO in $(jq -r 'keys[]' "$TEMP_DIR/utxos.json"); do
  TX_HASH=$(echo "$UTXO" | cut -d'#' -f1)
  TX_IX=$(echo "$UTXO" | cut -d'#' -f2)
  UTXO_AMOUNT=$(jq -r ".\"$UTXO\".value.lovelace" "$TEMP_DIR/utxos.json")
  TX_INS+=(--tx-in "$TX_HASH#$TX_IX")
  TOTAL_LOVELACE=$((TOTAL_LOVELACE + UTXO_AMOUNT))
done

log info "total balance: $(echo "scale=3; $TOTAL_LOVELACE / 1000000" | bc -l) ADA ($TOTAL_LOVELACE lovelace)"

# Convert ADA amount to lovelace
AMOUNT_LOVELACE=$(echo "$AMOUNT * 1000000" | bc | cut -d'.' -f1)

if [ "$TOTAL_LOVELACE" -lt "$AMOUNT_LOVELACE" ]; then
  echo >&2 "fatal: insufficient balance"
  exit 1
fi

# Build final transaction
FINAL_TX="$TEMP_DIR/tx.raw"
build_output=$(cardano-cli conway transaction build \
  "${TX_INS[@]}" \
  --tx-out "$RECEIVING_ADDRESS+$AMOUNT_LOVELACE" \
  --change-address "$MY_ADDRESS" \
  --out-file "$FINAL_TX")

if [[ $build_output == "Estimated transaction fee"* ]]; then
  log info "${build_output/Estimated/estimated}"
else
  log fatal "$build_output"
fi

# Sign the transaction
SIGNED_TX="$TEMP_DIR/tx.signed"
cardano-cli >&2 conway transaction sign \
  --tx-body-file "$FINAL_TX" \
  --signing-key-file "$TEMP_DIR/payment.skey" \
  --out-file "$SIGNED_TX"

log info "all done, outputting transaction CBOR to stdout…"

# Extract the cborHex from the signed transaction
jq -r '.cborHex' "$SIGNED_TX"
