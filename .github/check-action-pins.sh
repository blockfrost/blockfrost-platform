#!/usr/bin/env bash
#
# Verify that every third-party GitHub Action is pinned to an immutable
# commit SHA. Trusted orgs (whose supply-chain we accept) may use
# mutable tags.

set -euo pipefail

trusted_orgs=(actions github docker)

cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

trusted=$(
  IFS='|'
  echo "${trusted_orgs[*]/%//}"
)

targets=$(
  find .github/workflows -maxdepth 1 -name '*.yml' -o -name '*.yaml'
  find . -name action.yml -o -name action.yaml
)

violations=$(
  echo "$targets" | xargs grep -Hn 'uses:' |
    grep -vE "uses:\\s*(\\./|${trusted})" |
    grep -vP '@[0-9a-f]{40}\b' ||
    true
)

if [ -n "$violations" ]; then
  echo "::error::Third-party actions must be pinned to a full commit SHA."
  echo "$violations"
  exit 1
fi

echo "All third-party actions are properly SHA-pinned."
