#!/usr/bin/env bash

# Integration test: Platform + Gateway Hydra Micropayments
#
# This test verifies the full Hydra micropayment cycle between
# `blockfrost-platform` and `blockfrost-gateway` (with `dev_mock_db`):
#
# 1. Start the Gateway with a `[hydra_platform]` config section
# 2. Start the Platform pointing at the local Gateway (`--gateway-url`)
# 3. Wait for Hydra key exchange, Head Init, Commit, and Open
# 4. Send API requests through the Gateway → Platform
# 5. Verify that the Gateway triggers micropayments → Close → Fanout
# 6. Verify the cycle restarts ± indefinitely (2nd and 3rd fanout)

set -euo pipefail

# ---------------------------------------------------------------------------- #

work_dir=""
test_passed=false
gateway_pid=""
platform_pid=""
cleanup() {
  # Prevent re-entry on repeated Ctrl-C or cascading signals:
  trap '' INT TERM
  trap - EXIT

  if [ "$test_passed" = true ]; then
    echo >&2 "=== Test PASSED ==="
  else
    echo >&2 "=== Test FAILED ==="
  fi

  for pid in $gateway_pid $platform_pid; do
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
  echo >&2 "test:     $timestamp" "$level" "$@"
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

# How much the Gateway commits to the Hydra Head:
commit_ada=5.0
commit_lovelace=$(printf '%.0f' "$(echo "$commit_ada * 1000 * 1000" | bc)")
# How much each request is worth:
lovelace_per_request=$((1 * 1000 * 1000))
# How many requests to bundle for a microtransaction:
requests_per_microtransaction=2
# How many microtransactions until a fanout:
microtransactions_per_fanout=2
# How many fanout cycles to test (each is ~7 minutes):
num_fanout_cycles=3

requests_per_fanout=$((requests_per_microtransaction * microtransactions_per_fanout))

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

# We need 2 funded addresses:
# - gateway-hydra: pays L1 fees for the Gateway's hydra-node + commits funds to L2
# - platform-hydra: pays L1 fees for the Platform's hydra-node (empty commit)

log info "Verifying that ‘SUBMIT_MNEMONIC’ has enough funds…"

# MIN_FUEL_LOVELACE in Rust is 15 ADA. Each fanout cycle burns roughly one
# MIN_FUEL_LOVELACE in L1 fees for the Gateway and a bit less for the Platform,
# so fuel must scale with the number of cycles (plus one extra for headroom):
min_fuel_lovelace=$((15 * 1000 * 1000))
micropayments_total=$((num_fanout_cycles * requests_per_fanout * lovelace_per_request))

declare -A lovelace_fund
lovelace_fund["gateway-hydra"]=$((commit_lovelace + micropayments_total + (num_fanout_cycles + 1) * min_fuel_lovelace))
lovelace_fund["platform-hydra"]=$(((num_fanout_cycles + 1) * min_fuel_lovelace))

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
  log fatal "… insufficient funds on ‘SUBMIT_MNEMONIC’: have $(lovelace_to_ada "$submit_mnemonic_funds") ADA, need $(lovelace_to_ada "$required_funds") ADA."
  exit 1
else
  log info "… OK, have $(lovelace_to_ada "$submit_mnemonic_funds") ADA, need $(lovelace_to_ada "$required_funds") ADA."
fi

unset submit_mnemonic_funds
unset required_funds

# ---------------------------------------------------------------------------- #

log info "Generating L1 credentials…"

for participant in gateway-hydra platform-hydra platform-reward; do
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

log info "Funding L1 participants: gateway-hydra, platform-hydra"

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
    --tx-out "$(cat credentials/gateway-hydra/payment.addr)"+"${lovelace_fund["gateway-hydra"]}" \
    --tx-out "$(cat credentials/platform-hydra/payment.addr)"+"${lovelace_fund["platform-hydra"]}" \
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

for participant in gateway-hydra platform-hydra; do
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
platform_port=$(python3 -m portpicker)

gateway_url="http://127.0.0.1:${gateway_port}"
platform_secret="test-secret-at-least-8-chars"
platform_reward_address=$(cat credentials/platform-reward/payment.addr)
log info "Platform reward address (ephemeral): $platform_reward_address"

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

[hydra_platform]
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
platform_log="$work_dir/platform.log"

log info "Starting the Gateway (blockfrost-gateway--dev-mock-db)…"

blockfrost-gateway --config gateway-config.toml \
  > >(tee "$gateway_log" | sed -u 's/^/gateway:  /' >&2) 2>&1 &
gateway_pid=$!

sleep 1
wait4x http "${gateway_url}" --expect-status-code 200 --timeout 60s --interval 1s
log info "Gateway is up at ${gateway_url}"

# ---------------------------------------------------------------------------- #

log info "Starting the Platform (blockfrost-platform)…"

blockfrost-platform \
  --server-address 127.0.0.1 \
  --server-port "$platform_port" \
  --log-level "$log_level" \
  --node-socket-path "${CARDANO_NODE_SOCKET_PATH}" \
  --mode compact \
  --secret "$platform_secret" \
  --reward-address "$platform_reward_address" \
  --gateway-url "$gateway_url" \
  --hydra-cardano-signing-key "$(realpath credentials/platform-hydra/payment.sk)" \
  > >(tee "$platform_log" | sed -u 's/^/platform: /' >&2) 2>&1 &
platform_pid=$!

sleep 1
wait4x http "http://127.0.0.1:${platform_port}" --expect-status-code 200 --timeout 60s --interval 1s
log info "Platform is up at http://127.0.0.1:${platform_port}"

# ---------------------------------------------------------------------------- #

log info "Waiting for the Platform to register with the Gateway…"

# Poll the Gateway /stats endpoint (not a proxied route, so no request counting):
for _ in $(seq 1 60); do
  stats=$(curl -fsSL "${gateway_url}/stats" 2>/dev/null || echo '{}')
  relay_count=$(echo "$stats" | jq 'length' 2>/dev/null || echo "0")
  if ((relay_count > 0)); then
    log info "Platform is registered with the Gateway (active relays: $relay_count)"
    break
  fi
  log info "Waiting for Platform to register… (relays: $relay_count)"
  sleep 2
done

if ((relay_count == 0)); then
  log fatal "Platform never registered with the Gateway."
  exit 1
fi

# Extract the route UUID for the registered relay:
route_uuid=$(curl -fsSL "${gateway_url}/stats" | jq -r 'to_entries[0].value.api_prefix')
log info "Route UUID: $route_uuid"

# ---------------------------------------------------------------------------- #

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

# Apparently we need to wait 10 minutes for the Head to open between a
# `--blockfrost` Hydra node and a regular one. The future is now…

wait_for_gw_log_count 'waiting for the Open head status: status="Open"' 1 600 \
  "Hydra Head open (initial)" || exit 1

log info "Hydra Head is Open! Verifying the proxy route works…"

# Sanity check: one request through the Gateway to make sure the proxy works.
resp=$(curl -sS -w '\n%{http_code}' "${gateway_url}/${route_uuid}/" 2>/dev/null || true)
code="${resp##*$'\n'}"
if [ "$code" != "200" ]; then
  log fatal "Proxy route returned http/$code even though the head is open."
  exit 1
fi
log info "Proxy route OK (http/$code)"

# ---------------------------------------------------------------------------- #

# Track total requests sent so far (the sanity check above := 1).
total_requests_sent=1

# Number of head-open events seen so far (the initial open := 1).
head_opens_seen=1

# Number of "re-initializing" events expected (starts at 0).
reinits_seen=0

perform_fanout_cycle() {
  local fanout_num="$1"
  local is_last="$2" # 1 for the last cycle, empty otherwise

  log info "=== Fanout cycle $fanout_num: sending $requests_per_fanout requests ==="

  for nth in $(seq 1 "$requests_per_fanout"); do
    log info "Fanout $fanout_num: sending request $nth/$requests_per_fanout"

    resp=$(curl -sS -w '\n%{http_code}' "${gateway_url}/${route_uuid}/" 2>/dev/null || true)
    code="${resp##*$'\n'}"

    if [ "$code" != "200" ]; then
      log warn "Fanout $fanout_num: request $nth got http/$code, retrying in 2s…"
      sleep 2
      resp=$(curl -sS -w '\n%{http_code}' "${gateway_url}/${route_uuid}/" 2>/dev/null || true)
      code="${resp##*$'\n'}"
      if [ "$code" != "200" ]; then
        log fatal "Fanout $fanout_num: request $nth still failing: http/$code"
        exit 1
      fi
    fi

    log info "Fanout $fanout_num: request $nth: http/$code OK"
    total_requests_sent=$((total_requests_sent + 1))

    # Small delay between requests to let the L2 transactions settle:
    sleep 1
  done

  log info "Fanout $fanout_num: all $requests_per_fanout requests sent (total_requests_sent=$total_requests_sent)."

  # The contestation period is 60s, the invalidity period is (2+1)*60 = 180s.
  # We wait much longer for: Close + Fanout + Idle (180s) + re-Init + re-Commit + re-Open.
  fanout_wait_timeout=600

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
    log info "Fanout $fanout_num: head is Open again, ready for next cycle."
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

log info "Stopping Gateway (pid $gateway_pid) and Platform (pid $platform_pid)…"
kill "$gateway_pid" "$platform_pid" 2>/dev/null || true
wait "$gateway_pid" "$platform_pid" 2>/dev/null || true
log info "Gateway and Platform stopped."

log info "Waiting 30s for in-flight L1 transactions to clear the mempool…"
sleep 30

# ---------------------------------------------------------------------------- #

log info "Verifying that funds were transferred on L1 to the Platform reward address…"

# The minimum expected amount is `num_fanout_cycles * requests_per_fanout *
# lovelace_per_request`:
expected_lovelace=$((num_fanout_cycles * requests_per_fanout * lovelace_per_request))

# The third fanout settles on L1 asynchronously; let’s wait longer for the UTxOs to appear:
l1_verify_timeout=300
l1_verify_start=$(date +%s)
platform_l1_lovelace=0

while true; do
  platform_l1_lovelace=$(cardano-cli query utxo \
    --address "$platform_reward_address" \
    --out-file /dev/stdout |
    jq '[.[] | .value.lovelace] | add // 0')

  log info "Platform reward address has $(lovelace_to_ada "$platform_l1_lovelace") ADA ($platform_l1_lovelace lovelace), expected at least $(lovelace_to_ada "$expected_lovelace") ADA ($expected_lovelace lovelace)"

  if ((platform_l1_lovelace >= expected_lovelace)); then
    log info "… OK, L1 fund transfer verified! Platform received at least $(lovelace_to_ada "$expected_lovelace") ADA."
    break
  fi

  elapsed=$(($(date +%s) - l1_verify_start))
  if ((elapsed > l1_verify_timeout)); then
    log fatal "Timed out waiting for L1 funds at Platform reward address (${l1_verify_timeout}s). Got: $platform_l1_lovelace lovelace, expected: $expected_lovelace"
    exit 1
  fi

  sleep 10
done

# ---------------------------------------------------------------------------- #

log info "Returning all funds to ‘SUBMIT_MNEMONIC’…"

txdir=tx-02-return-test-ada
mkdir -p "$txdir"
change_address=$(cat credentials/submit-mnemonic/payment.addr)

declare -A lovelace_remaining

for participant in gateway-hydra platform-hydra platform-reward; do
  addr=$(cat credentials/"$participant"/payment.addr)
  utxo_json=$(cardano-cli query utxo --address "$addr" --out-file /dev/stdout)
  funds=$(echo "$utxo_json" | jq '[.[] | .value.lovelace] | add // 0')
  lovelace_remaining["$participant"]=$funds
  log info "Returning funds from $participant ($funds lovelace)…"

  if ((funds == 0)); then
    log warn "$participant has no funds to return; skipping."
    continue
  fi

  tx_ins=$(echo "$utxo_json" | jq -j 'to_entries[].key | "--tx-in ", ., " "')

  # shellcheck disable=SC2086
  cardano-cli latest transaction build \
    $tx_ins \
    --change-address "$change_address" \
    --out-file "$txdir/tx-$participant.json"
  cardano-cli latest transaction sign \
    --tx-file "$txdir/tx-$participant.json" \
    --signing-key-file "credentials/$participant/payment.sk" \
    --out-file "$txdir/tx-signed-$participant.json"
  cardano-cli latest transaction submit --tx-file "$txdir/tx-signed-$participant.json"
  log info "Returned funds from $participant."
done

# ---------------------------------------------------------------------------- #

log info "Calculating how much was lost in Hydra transaction fees (excluding L1 fees from and to ‘SUBMIT_MNEMONIC’)…"

total_cost=0

for participant in gateway-hydra platform-hydra; do
  cost=$((lovelace_fund["$participant"] - lovelace_remaining["$participant"]))
  total_cost=$((total_cost + cost))
  log info "Address '$participant' lost $(lovelace_to_ada "$cost") ADA."
done

# `platform-reward` was not funded, as it received micropayments from `gateway-hydra`
pr_received=${lovelace_remaining["platform-reward"]:-0}
log info "Address 'platform-reward' received $(lovelace_to_ada "$pr_received") ADA in micropayments."

# The net fee cost is total spent by funded addresses minus micropayments received
# (micropayments transfer from `gateway-hydra` to `platform-reward`, so they cancel out).
total_fees=$((total_cost - pr_received))
log warn "In total, we lost $(lovelace_to_ada "$total_fees") ADA (in Hydra and L1 transaction fees)."

# ---------------------------------------------------------------------------- #

test_passed=true
log info "Test passed! Exiting."
