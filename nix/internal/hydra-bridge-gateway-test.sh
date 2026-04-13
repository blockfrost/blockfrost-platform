#!/usr/bin/env bash

# Integration test: SDK Bridge + Gateway Hydra Micropayments
#
# This test verifies the full Hydra micropayment cycle between
# `blockfrost-sdk-bridge` (sender) and `blockfrost-gateway` (with
# `dev_mock_db`, receiver):
#
# 1. Start the Gateway with a `[hydra_bridge]` config section
# 2. Start the SDK Bridge pointing at the local Gateway (`--gateway-ws-url`)
# 3. Wait for Hydra key exchange, Head Init, Commit, and Open
# 4. Send API requests through the SDK Bridge -> Gateway
# 5. Verify that the Gateway triggers Close -> Fanout after enough micropayments
# 6. Verify the cycle restarts ± indefinitely (2nd and 3rd fanout)

set -euo pipefail

# ---------------------------------------------------------------------------- #

work_dir=""
test_passed=false
gateway_pid=""
bridge_pid=""
cleanup() {
  # Prevent re-entry on repeated Ctrl-C or cascading signals:
  trap '' INT TERM
  trap - EXIT

  if [ "$test_passed" = true ]; then
    echo >&2 "=== Test PASSED ==="
  else
    echo >&2 "=== Test FAILED ==="
  fi

  for pid in $gateway_pid $bridge_pid; do
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      echo >&2 "Sending SIGTERM to pid $pid"
      kill -TERM "$pid" 2>/dev/null || true
    fi
  done
  wait 2>/dev/null || true

  if [ -n "$work_dir" ]; then
    cd /
    rm -rf -- "$work_dir"
  fi
  if [ "$test_passed" = true ]; then exit 0; else exit 1; fi
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
  echo >&2 "test:    $timestamp" "$level" "$@"
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
for v in NETWORK BLOCKFROST_PROJECT_ID CARDANO_NODE_SOCKET_PATH SUBMIT_MNEMONIC CARDANO_NODE_NETWORK_ID; do
  require_env "$v"
done
if ((missing)); then
  exit 1
fi

lovelace_to_ada() {
  printf '%d.%06d' $(($1 / (1000 * 1000))) $(($1 % (1000 * 1000)))
}

# ---------------------------------------------------------------------------- #

# How much the Bridge commits to the Hydra Head:
commit_ada=7.0
commit_lovelace=$(printf '%.0f' "$(echo "$commit_ada * 1000 * 1000" | bc)")
# How much each request is worth:
lovelace_per_request=$((1 * 1000 * 1000))
# How many requests to bundle for a microtransaction:
requests_per_microtransaction=2
# How many microtransactions until a fanout (the Gateway counts the prepay as
# microtransaction #1, so only microtransactions_per_fanout-1 are request-driven):
microtransactions_per_fanout=3
# How many fanout cycles to test (each can take up to ~20 minutes on a slow
# Preview testnet day due to L1 confirmation times):
num_fanout_cycles=3

# The prepay counts as microtransaction #1 for the Gateway's fanout trigger,
# so the number of HTTP-request-driven microtransactions per cycle is one fewer:
requests_per_fanout=$((requests_per_microtransaction * (microtransactions_per_fanout - 1)))

# ---------------------------------------------------------------------------- #

work_dir=$(mktemp -d)
cd "$work_dir"

export HOME="$work_dir"
unset XDG_CONFIG_HOME XDG_CACHE_HOME XDG_DATA_HOME XDG_STATE_HOME

log info "Working directory (and HOME): $work_dir"
log info "Network tip: $(cardano-cli query tip | jq --compact-output .)"

mkdir -p credentials

# ---------------------------------------------------------------------------- #

# Derive keys using cardano-address
log info "Deriving keys from the 'SUBMIT_MNEMONIC'"

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

  log info "'SUBMIT_MNEMONIC' address: $(cat "payment.addr")"
)

# ---------------------------------------------------------------------------- #

# We need 2 funded addresses:
# - bridge-hydra: pays L1 fees for the Bridge's hydra-node + commits funds to L2
# - gateway-hydra: pays L1 fees for the Gateway's hydra-node (empty commit)

log info "Verifying that 'SUBMIT_MNEMONIC' has enough funds…"

# MIN_FUEL_LOVELACE in Rust is 15 ADA. Each fanout cycle burns roughly one
# MIN_FUEL_LOVELACE in L1 fees for the Bridge and a bit less for the Gateway,
# so fuel must scale with the number of cycles (plus one extra for headroom):
min_fuel_lovelace=$((15 * 1000 * 1000))
# Total micropayments per cycle includes the prepay (microtransactions_per_fanout
# counts it), so use the full microtransactions_per_fanout here:
micropayments_per_cycle=$((microtransactions_per_fanout * requests_per_microtransaction * lovelace_per_request))
micropayments_total=$((num_fanout_cycles * micropayments_per_cycle))

declare -A lovelace_fund
lovelace_fund["bridge-hydra"]=$((commit_lovelace + micropayments_total + (num_fanout_cycles + 1) * min_fuel_lovelace))
lovelace_fund["gateway-hydra"]=$(((num_fanout_cycles + 1) * min_fuel_lovelace))

submit_mnemonic_funds=$(cardano-cli query utxo \
  --address "$(cat credentials/submit-mnemonic/payment.addr)" \
  --out-file /dev/stdout |
  jq '[.[] | .value.lovelace] | add // 0')

# 1 ADA extra for tx fees:
required_funds=$((1 * 1000 * 1000))
for k in "${!lovelace_fund[@]}"; do
  ((required_funds += lovelace_fund[$k]))
done

if ((submit_mnemonic_funds < required_funds)); then
  log fatal "… insufficient funds on 'SUBMIT_MNEMONIC': have $(lovelace_to_ada "$submit_mnemonic_funds") ADA, need $(lovelace_to_ada "$required_funds") ADA."
  exit 1
else
  log info "… OK, have $(lovelace_to_ada "$submit_mnemonic_funds") ADA, need $(lovelace_to_ada "$required_funds") ADA."
fi

unset submit_mnemonic_funds
unset required_funds

# ---------------------------------------------------------------------------- #

log info "Generating L1 credentials…"

for participant in gateway-hydra bridge-hydra; do
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

log info "Funding L1 participants: bridge-hydra, gateway-hydra"

txdir=tx-01-fund-participants
mkdir -p $txdir

max_funding_attempts=5
funding_retry_delay=25
for funding_attempt in $(seq 1 $max_funding_attempts); do
  log info "Funding attempt $funding_attempt/$max_funding_attempts"

  cardano-cli query utxo \
    --address "$(cat credentials/submit-mnemonic/payment.addr)" \
    --out-file $txdir/input-utxo.json

  # shellcheck disable=SC2046
  if cardano-cli latest transaction build \
    $(jq -r 'keys[]' <$txdir/input-utxo.json | shuf | head -n 200 | sed 's/^/--tx-in /') \
    --change-address "$(cat credentials/submit-mnemonic/payment.addr)" \
    --tx-out "$(cat credentials/bridge-hydra/payment.addr)"+"${lovelace_fund["bridge-hydra"]}" \
    --tx-out "$(cat credentials/gateway-hydra/payment.addr)"+"${lovelace_fund["gateway-hydra"]}" \
    --out-file $txdir/tx.json &&
    cardano-cli latest transaction sign \
      --tx-file $txdir/tx.json \
      --signing-key-file credentials/submit-mnemonic/payment.sk \
      --out-file $txdir/tx-signed.json &&
    cardano-cli latest transaction submit --tx-file $txdir/tx-signed.json; then
    log info "Funding transaction submitted successfully."
    break
  fi

  if ((funding_attempt == max_funding_attempts)); then
    log fatal "All $max_funding_attempts funding attempts failed."
    exit 1
  fi

  log warn "Funding attempt $funding_attempt failed, retrying in ${funding_retry_delay}s (waiting for a new block)…"
  sleep "$funding_retry_delay"
done

# ---------------------------------------------------------------------------- #

for participant in bridge-hydra gateway-hydra; do
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

log_level=info

gateway_port=$(python3 -m portpicker)
bridge_port=$(python3 -m portpicker)

gateway_url="http://127.0.0.1:${gateway_port}"
bridge_url="http://127.0.0.1:${bridge_port}"

# ---------------------------------------------------------------------------- #

log info "Writing Gateway config…"

cat >gateway-config.toml <<EOF
[server]
address = '127.0.0.1:${gateway_port}'
log_level = '${log_level}'

[database]
connection_string = 'postgresql://not-used-with-dev-mock-db@localhost/dummy'

[blockfrost]
project_id = '${BLOCKFROST_PROJECT_ID}'
nft_asset = '4213fc3eac8c781ac85514dd1de9aaabcd5a3a81cc2df4f413b9b295'

[hydra_bridge]
max_concurrent_hydra_nodes = 2
cardano_signing_key = '$(realpath credentials/gateway-hydra/payment.sk)'
commit_ada = ${commit_ada}
lovelace_per_request = ${lovelace_per_request}
requests_per_microtransaction = ${requests_per_microtransaction}
microtransactions_per_fanout = ${microtransactions_per_fanout}
EOF

cat gateway-config.toml
log info "Gateway config written."

# ---------------------------------------------------------------------------- #

gateway_log="$work_dir/gateway.log"
bridge_log="$work_dir/bridge.log"

log info "Starting the Gateway (blockfrost-gateway--dev-mock-db)…"

blockfrost-gateway --config gateway-config.toml \
  > >(tee "$gateway_log" | sed -u 's/^/gateway: /' >&2) 2>&1 &
gateway_pid=$!

sleep 1
wait4x http "${gateway_url}" --expect-status-code 200 --timeout 60s --interval 1s
log info "Gateway is up at ${gateway_url}"

# ---------------------------------------------------------------------------- #

log info "Starting the SDK Bridge (blockfrost-sdk-bridge)…"

blockfrost-sdk-bridge \
  --gateway-ws-url "${gateway_url}" \
  --listen-address "127.0.0.1:${bridge_port}" \
  --network "${NETWORK}" \
  --blockfrost-project-id "${BLOCKFROST_PROJECT_ID}" \
  --cardano-signing-key "$(realpath credentials/bridge-hydra/payment.sk)" \
  > >(tee "$bridge_log" | sed -u 's/^/bridge:  /' >&2) 2>&1 &
bridge_pid=$!

sleep 1
wait4x tcp "127.0.0.1:${bridge_port}" --timeout 60s --interval 1s
log info "SDK Bridge is listening at ${bridge_url}"

# ---------------------------------------------------------------------------- #

# Helper: count occurrences of a pattern in the Bridge log.
bridge_log_count() {
  local n
  n=$(grep -c "$1" "$bridge_log" 2>/dev/null) || true
  echo "${n:-0}"
}

# Helper: wait for a pattern to appear at least N times in the Bridge log.
# Usage: wait_for_bridge_log_count <pattern> <min_count> <timeout_seconds> <description>
wait_for_bridge_log_count() {
  local pattern="$1"
  local min_count="$2"
  local timeout="$3"
  local desc="$4"
  local start elapsed count

  start=$(date +%s)
  while true; do
    count=$(bridge_log_count "$pattern")
    elapsed=$(($(date +%s) - start))
    log info "$desc: occurrences=$count, need=$min_count (elapsed: ${elapsed}s)"
    if ((count >= min_count)); then
      return 0
    fi
    if ((elapsed > timeout)); then
      log fatal "$desc: timed out after ${timeout}s (occurrences=$count, need=$min_count)"
      return 1
    fi
    sleep 1
  done
}

# Helper: count occurrences of a pattern in the Gateway log.
gw_log_count() {
  local n
  n=$(grep -c "$1" "$gateway_log" 2>/dev/null) || true
  echo "${n:-0}"
}

# Helper: wait for a pattern to appear at least N times in the Gateway log.
# Usage: wait_for_gw_log_count <pattern> <min_count> <timeout_seconds> <description>
wait_for_gw_log_count() {
  local pattern="$1"
  local min_count="$2"
  local timeout="$3"
  local desc="$4"
  local start elapsed count

  start=$(date +%s)
  while true; do
    count=$(gw_log_count "$pattern")
    elapsed=$(($(date +%s) - start))
    log info "$desc: occurrences=$count, need=$min_count (elapsed: ${elapsed}s)"
    if ((count >= min_count)); then
      return 0
    fi
    if ((elapsed > timeout)); then
      log fatal "$desc: timed out after ${timeout}s (occurrences=$count, need=$min_count)"
      return 1
    fi
    sleep 5
  done
}

# ---------------------------------------------------------------------------- #

log info "Waiting for the Hydra Head to become Open…"

# The initial Head opening can take several minutes on the Cardano L1 (init +
# commit + open transactions need to be confirmed):

wait_for_gw_log_count 'waiting for the Open head status: status="Open"' 1 1200 \
  "Hydra Head open (initial)" || exit 1

log info "Hydra Head is Open! Waiting for the Bridge to have request credits…"

# After the head opens the Bridge sends a prepay microtransaction and then polls
# its local hydra-node snapshot every 1 s to detect the confirmed balance change.
# It logs "req. credits +N" once credits are granted.  We wait for that before
# sending traffic so we never fire a request that would get a 402.
bridge_credits_seen=1
# The Bridge has its own hydra-node connection and may lag behind the Gateway in
# detecting the Open status by 20+ seconds. On top of that, it schedules the
# prepay with a 15 s delay and then waits for snapshot confirmation. Use a
# generous timeout (90 s) so that 30 s is never too tight on slow CI.
wait_for_bridge_log_count "req. credits +" "$bridge_credits_seen" 90 \
  "Bridge credits (initial)" || exit 1

log info "Bridge has credits. Ready to send traffic."

# Track how many credits the Bridge currently has so we can wait for
# replenishment before sending the next request when credits are exhausted.
# Each "req. credits +" log grants requests_per_microtransaction credits.
test_credits=$requests_per_microtransaction

# ---------------------------------------------------------------------------- #

# Track total requests sent so far.
total_requests_sent=0

# Number of head-open events seen so far (the initial open := 1).
head_opens_seen=1

# Number of "re-initializing" events expected (starts at 0).
reinits_seen=0

perform_fanout_cycle() {
  local fanout_num="$1"
  local is_last="$2" # 1 for the last cycle, empty otherwise

  log info "=== Fanout cycle $fanout_num: sending $requests_per_fanout requests ==="

  for nth in $(seq 1 "$requests_per_fanout"); do
    # The Bridge only has requests_per_microtransaction credits at a time.
    # When exhausted a new microtransaction is sent but takes ~1-2s to confirm
    # in the snapshot before the Bridge grants itself fresh credits.  Wait for
    # the next "req. credits +" log rather than hitting a 402.
    if ((test_credits <= 0)); then
      bridge_credits_seen=$((bridge_credits_seen + 1))
      log info "Fanout $fanout_num: waiting for Bridge credit grant #$bridge_credits_seen before request $nth…"
      wait_for_bridge_log_count "req. credits +" "$bridge_credits_seen" 30 \
        "Fanout $fanout_num: credits before request $nth" || exit 1
      test_credits=$((test_credits + requests_per_microtransaction))
    fi

    log info "Fanout $fanout_num: sending request $nth/$requests_per_fanout (test_credits=$test_credits)"

    resp=$(curl -sS -w '\n%{http_code}' "${bridge_url}/" 2>/dev/null || true)
    code="${resp##*$'\n'}"

    if [ "$code" != "200" ]; then
      log fatal "Fanout $fanout_num: request $nth failed: http/$code"
      exit 1
    fi

    log info "Fanout $fanout_num: request $nth: http/$code OK"
    total_requests_sent=$((total_requests_sent + 1))
    test_credits=$((test_credits - 1))

    # Small delay between requests to let the L2 transactions settle:
    sleep 1
  done

  log info "Fanout $fanout_num: all $requests_per_fanout requests sent (total_requests_sent=$total_requests_sent)."

  # The contestation period is 60s, the invalidity period is (2+1)*60 = 180s.
  # We wait much longer for: Close + Fanout + Idle (180s) + re-Init + re-Commit + re-Open.
  fanout_wait_timeout=1200

  # Wait for the Gateway log to show "re-initializing the Hydra Head" for the
  # Nth time, which means the Nth fanout completed and it's cycling back:
  reinits_seen=$((reinits_seen + 1))
  log info "Fanout $fanout_num: waiting for re-initialization #$reinits_seen in Gateway log (up to ${fanout_wait_timeout}s)…"
  wait_for_gw_log_count "re-initializing the Hydra Head" "$reinits_seen" "$fanout_wait_timeout" \
    "Fanout $fanout_num: Close → Fanout → Re-init" || exit 1

  log info "Fanout $fanout_num: fanout cycle completed on L2!"

  # For non-last cycles, also wait for the head to reopen, so we can send the
  # next batch of requests:
  if [[ -z $is_last ]]; then
    head_opens_seen=$((head_opens_seen + 1))
    log info "Fanout $fanout_num: waiting for the head to reopen (Open #$head_opens_seen)…"
    wait_for_gw_log_count 'waiting for the Open head status: status="Open"' "$head_opens_seen" "$fanout_wait_timeout" \
      "Fanout $fanout_num: head reopen" || exit 1
    log info "Fanout $fanout_num: head is Open again. Waiting for Bridge credits…"
    # Snapshot the current count so we wait for a genuinely NEW credit grant
    # from the new head session (stale entries from previous cycles must not
    # satisfy the check since credits_available is reset on head close).
    bridge_credits_seen=$(($(bridge_log_count "req. credits +") + 1))
    # Same lag as above: Bridge may take 20+ s to see Open + 15 s prepay delay.
    wait_for_bridge_log_count "req. credits +" "$bridge_credits_seen" 90 \
      "Fanout $fanout_num: Bridge credits" || exit 1
    log info "Fanout $fanout_num: Bridge has credits, ready for next cycle."
    test_credits=$requests_per_microtransaction
  fi
}

# ---------------------------------------------------------------------------- #

for cycle in $(seq 1 "$num_fanout_cycles"); do
  if ((cycle == num_fanout_cycles)); then
    is_last=1
  else
    is_last=
  fi
  log info "=== Starting fanout cycle $cycle/$num_fanout_cycles ==="
  perform_fanout_cycle "$cycle" "$is_last"
done

# ---------------------------------------------------------------------------- #

log info "All $num_fanout_cycles fanout cycles completed successfully!"

log info "Stopping Gateway (pid $gateway_pid) and SDK Bridge (pid $bridge_pid)…"
kill "$gateway_pid" "$bridge_pid" 2>/dev/null || true
wait "$gateway_pid" "$bridge_pid" 2>/dev/null || true
log info "Gateway and SDK Bridge stopped."

log info "Waiting 30s for in-flight L1 transactions to clear the mempool…"
sleep 30

# ---------------------------------------------------------------------------- #

log info "Verifying that funds were transferred on L1 to the Gateway…"

# The minimum expected amount is the total micropayments received (including
# prepays).  The Gateway's signing key address also holds remaining L1 fuel,
# so the actual balance will be higher:
expected_lovelace=$micropayments_total

# The third fanout settles on L1 asynchronously; let's wait longer for the UTxOs to appear:
l1_verify_timeout=300
l1_verify_start=$(date +%s)
gateway_l1_lovelace=0

while true; do
  gateway_l1_lovelace=$(cardano-cli query utxo \
    --address "$(cat credentials/gateway-hydra/payment.addr)" \
    --out-file /dev/stdout |
    jq '[.[] | .value.lovelace] | add // 0')

  log info "Gateway address has $(lovelace_to_ada "$gateway_l1_lovelace") ADA ($gateway_l1_lovelace lovelace), expected at least $(lovelace_to_ada "$expected_lovelace") ADA ($expected_lovelace lovelace)"

  if ((gateway_l1_lovelace >= expected_lovelace)); then
    log info "… OK, L1 fund transfer verified! Gateway has at least $(lovelace_to_ada "$expected_lovelace") ADA."
    break
  fi

  elapsed=$(($(date +%s) - l1_verify_start))
  if ((elapsed > l1_verify_timeout)); then
    log fatal "Timed out waiting for L1 funds at Gateway address (${l1_verify_timeout}s). Got: $gateway_l1_lovelace lovelace, expected: $expected_lovelace"
    exit 1
  fi

  sleep 10
done

# ---------------------------------------------------------------------------- #

log info "Returning all funds to 'SUBMIT_MNEMONIC'…"

txdir=tx-02-return-test-ada
mkdir -p "$txdir"
change_address=$(cat credentials/submit-mnemonic/payment.addr)

declare -A lovelace_remaining

max_return_attempts=5
return_retry_delay=25

for participant in bridge-hydra gateway-hydra; do
  for return_attempt in $(seq 1 $max_return_attempts); do
    addr=$(cat credentials/"$participant"/payment.addr)
    utxo_json=$(cardano-cli query utxo --address "$addr" --out-file /dev/stdout)
    funds=$(echo "$utxo_json" | jq '[.[] | .value.lovelace] | add // 0')
    lovelace_remaining["$participant"]=$funds
    log info "Returning funds from $participant ($funds lovelace)…"

    if ((funds == 0)); then
      log warn "$participant has no funds to return; skipping."
      break
    fi

    tx_ins=$(echo "$utxo_json" | jq -j 'to_entries[].key | "--tx-in ", ., " "')

    # shellcheck disable=SC2086
    if cardano-cli latest transaction build \
      $tx_ins \
      --change-address "$change_address" \
      --out-file "$txdir/tx-$participant.json" &&
      cardano-cli latest transaction sign \
        --tx-file "$txdir/tx-$participant.json" \
        --signing-key-file "credentials/$participant/payment.sk" \
        --out-file "$txdir/tx-signed-$participant.json" &&
      cardano-cli latest transaction submit --tx-file "$txdir/tx-signed-$participant.json"; then
      log info "Returned funds from $participant."
      break
    fi

    if ((return_attempt == max_return_attempts)); then
      log fatal "All $max_return_attempts attempts to return funds from $participant failed."
      exit 1
    fi

    log warn "Return attempt $return_attempt for $participant failed, retrying in ${return_retry_delay}s (waiting for a new block)…"
    sleep "$return_retry_delay"
  done
done

# ---------------------------------------------------------------------------- #

log info "Calculating how much was lost in Hydra transaction fees (excluding L1 fees from and to 'SUBMIT_MNEMONIC')…"

total_cost=0

for participant in bridge-hydra gateway-hydra; do
  cost=$((lovelace_fund["$participant"] - lovelace_remaining["$participant"]))
  total_cost=$((total_cost + cost))
  if ((cost >= 0)); then
    log info "Address '$participant' spent $(lovelace_to_ada "$cost") ADA (net)."
  else
    gain=$((-cost))
    log info "Address '$participant' gained $(lovelace_to_ada "$gain") ADA (net, from micropayments)."
  fi
done

# The net cost across both participants equals the total L1 transaction and
# Hydra fees, since micropayments transfer from bridge-hydra to gateway-hydra
# and cancel out:
log warn "In total, we lost $(lovelace_to_ada "$total_cost") ADA (in Hydra and L1 transaction fees)."

# ---------------------------------------------------------------------------- #

test_passed=true
log info "Test passed! Exiting."
