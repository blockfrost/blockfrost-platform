#!/usr/bin/env bash

# Integration test: Platform + 2× Gateway HA (High Availability)
#
# Tests that a single `blockfrost-platform` instance can register with one
# `blockfrost-gateway` and, thanks to `peer_urls` + `peer_secret,` connect via
# WebSocket to multiple Gateway peers simultaneously. Requests proxied through
# any peer Gateway reach the same Platform instance.
#
# 1. Start two Gateways (A and B) configured as peers (`peer_urls`, `peer_secret`)
# 2. Start one Platform pointing at Gateway A (`--gateway-url`)
# 3. Wait for the Platform to register and appear on both Gateways
# 4. Send 2 requests through Gateway A and 2 through Gateway B
# 5. Verify all requests succeed (HTTP 200, proxied to the Platform)

set -euo pipefail

# ---------------------------------------------------------------------------- #

work_dir=""
test_passed=false
gateway_a_pid=""
gateway_b_pid=""
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

  for pid in $gateway_a_pid $gateway_b_pid $platform_pid; do
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
  echo >&2 "test:      $timestamp" "$level" "$@"
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
# shellcheck disable=SC2043
for v in CARDANO_NODE_SOCKET_PATH; do
  require_env "$v"
done
if ((missing)); then
  exit 1
fi

# ---------------------------------------------------------------------------- #

work_dir=$(mktemp -d)
cd "$work_dir"

log info "Working directory: $work_dir"

gateway_a_port=$(python3 -m portpicker)
gateway_b_port=$(python3 -m portpicker)
platform_port=$(python3 -m portpicker)

gateway_a_url="http://127.0.0.1:${gateway_a_port}"
gateway_b_url="http://127.0.0.1:${gateway_b_port}"
platform_url="http://127.0.0.1:${platform_port}"

peer_secret="ha-test-peer-secret"
platform_secret="test-secret-at-least-8-chars"
reward_address="addr_test1vqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqd9tg5t"
log_level=info

# ---------------------------------------------------------------------------- #

log info "Writing Gateway A config (port ${gateway_a_port})…"

cat >gateway-a.toml <<EOF
[server]
address = '127.0.0.1:${gateway_a_port}'
log_level = '${log_level}'
url = '${gateway_a_url}'
peer_urls = ['${gateway_a_url}', '${gateway_b_url}']
peer_secret = '${peer_secret}'

[database]
connection_string = 'postgresql://not-used-with-dev-mock-db@localhost/dummy'

[blockfrost]
project_id = 'preview00000000000000000000000000'
nft_asset = '4213fc3eac8c781ac85514dd1de9aaabcd5a3a81cc2df4f413b9b295'
EOF

log info "Writing Gateway B config (port ${gateway_b_port})…"

cat >gateway-b.toml <<EOF
[server]
address = '127.0.0.1:${gateway_b_port}'
log_level = '${log_level}'
url = '${gateway_b_url}'
peer_urls = ['${gateway_a_url}', '${gateway_b_url}']
peer_secret = '${peer_secret}'

[database]
connection_string = 'postgresql://not-used-with-dev-mock-db@localhost/dummy'

[blockfrost]
project_id = 'preview00000000000000000000000000'
nft_asset = '4213fc3eac8c781ac85514dd1de9aaabcd5a3a81cc2df4f413b9b295'
EOF

# ---------------------------------------------------------------------------- #

gateway_a_log="$work_dir/gateway-a.log"
gateway_b_log="$work_dir/gateway-b.log"
platform_log="$work_dir/platform.log"

log info "Starting Gateway A…"

blockfrost-gateway --config gateway-a.toml \
  > >(tee "$gateway_a_log" | sed -u 's/^/gateway-A: /' >&2) 2>&1 &
gateway_a_pid=$!

log info "Starting Gateway B…"

blockfrost-gateway --config gateway-b.toml \
  > >(tee "$gateway_b_log" | sed -u 's/^/gateway-B: /' >&2) 2>&1 &
gateway_b_pid=$!

sleep 1
wait4x http "${gateway_a_url}" --expect-status-code 200 --timeout 60s --interval 1s
log info "Gateway A is up at ${gateway_a_url}"
wait4x http "${gateway_b_url}" --expect-status-code 200 --timeout 60s --interval 1s
log info "Gateway B is up at ${gateway_b_url}"

# ---------------------------------------------------------------------------- #

# shellcheck disable=SC2016
log info 'Starting the Platform (`blockfrost-platform`)…'

blockfrost-platform \
  --server-address 127.0.0.1 \
  --server-port "$platform_port" \
  --log-level "$log_level" \
  --node-socket-path "${CARDANO_NODE_SOCKET_PATH}" \
  --mode compact \
  --secret "$platform_secret" \
  --reward-address "$reward_address" \
  --gateway-url "$gateway_a_url" \
  > >(tee "$platform_log" | sed -u 's/^/platform:  /' >&2) 2>&1 &
platform_pid=$!

sleep 1
wait4x http "${platform_url}" --expect-status-code 200 --timeout 60s --interval 1s
log info "Platform is up at ${platform_url}"

# ---------------------------------------------------------------------------- #

declare -A gateway_url
gateway_url["A"]=$gateway_a_url
gateway_url["B"]=$gateway_b_url

for gw in A B; do
  log info "Waiting for the Platform to register with Gateway ${gw}…"

  api_prefix=""
  for _ in $(seq 1 60); do
    api_prefix=$(curl -fsSL "${gateway_url[$gw]}/stats" 2>/dev/null | jq -r 'to_entries | .[0].value.api_prefix // empty') || true
    if [[ -n $api_prefix ]]; then
      break
    fi
    sleep 2
  done

  if [[ -z $api_prefix ]]; then
    log fatal "Platform never registered with Gateway ${gw}."
    exit 1
  fi

  log info "Platform registered with Gateway ${gw}. Route UUID: $api_prefix"
done

# ---------------------------------------------------------------------------- #

for gw in A B; do
  log info "Sending 2 requests through Gateway ${gw} (route: /${api_prefix}/)…"

  for i in 1 2; do
    resp=$(curl -sS -w '\n%{http_code}' "${gateway_url[$gw]}/${api_prefix}/" 2>/dev/null || true)
    code="${resp##*$'\n'}"
    if [ "$code" != "200" ]; then
      log fatal "Gateway ${gw}: request $i failed with http/$code"
      exit 1
    fi
    log info "Gateway ${gw}: request $i: http/$code OK"
    sleep 0.5
  done
done

# ---------------------------------------------------------------------------- #

log info "All 4 requests succeeded through both gateways!"

test_passed=true
log info "Test passed! Exiting."
