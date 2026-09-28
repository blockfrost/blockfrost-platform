# Blockfrost Platform Docs

https://platform.blockfrost.io

## Development

Before you start make sure you have downloaded and installed [Node.js LTS](https://nodejs.org/en/download/), [pnpm](https://pnpm.io/installation) and git.

1. install dependencies `pnpm install`
2. `pnpm dev`

## Production

Deployemnts are done by Vercel. Use UI to deploy new version.

## Rust coverage

To make a coverage report for the unit tests, do these steps:

1. Run the unit tests with coverage instrumentation:

   ```shell
   nix run .#internal.x86_64-linux.coverage-unit-tests
   ```

   This command writes `unit.lcov` to the current directory.

2. Make the coverage report from `unit.lcov`:

   ```shell
   nix run .#internal.x86_64-linux.coverage-report -- unit.lcov
   ```

The report contains `coverage-html/index.html` and `coverage-html/combined.lcov`.
You must give each input file as an argument. `coverage-report` does not search the directory for input files.
To merge coverage, give more than one `.lcov` file.
All input files must come from the same source revision as the checkout.

Each integration runner needs a socket of a Cardano node, API credentials, and an endpoint of the data node:

- `coverage-platform-integ-tests` uses `CARDANO_NODE_SOCKET_PATH`, `BLOCKFROST_PREVIEW_PROJECT_ID`, and `DATA_NODE_ENDPOINT`.
- `coverage-blockfrost-tests-{network}` uses `CARDANO_NODE_SOCKET_PATH`, `PROJECT_ID`, `SUBMIT_MNEMONIC`, and `DATA_NODE_ENDPOINT`.
  The supported networks are `preview`, `preprod`, and `mainnet`.

To make coverage for the Blockfrost tests on `preview`, do these steps:

1. Remove the `blockfrost-preview-*.profraw` files from earlier runs.
2. Set `LLVM_PROFILE_FILE` to an absolute path.
3. Run the tests.
4. Convert the profiles to LCOV.

```shell
export LLVM_PROFILE_FILE="$PWD/blockfrost-preview-%p-%m.profraw"
nix run .#internal.x86_64-linux.coverage-blockfrost-tests-preview
nix run .#internal.x86_64-linux.coverage-convert-blockfrost-preview
```

The converter writes `blockfrost-preview.lcov`.
For `preprod` or `mainnet`, use the name of that network in the file names and commands.

The `blockfrost-tests-{network}` and `blockfrost-ignore-check-{network}` runners use binaries without instrumentation.
The `hydra-platform-gateway-test` and `hydra-bridge-gateway-test` runners also use binaries without instrumentation.
The `coverage-hydra-platform-gateway-test` and `coverage-hydra-bridge-gateway-test` runners use instrumented binaries.
Their converters are `coverage-convert-hydra-pg` and `coverage-convert-hydra-bg`.
These converters read the profiles with the prefixes `hydra-pg` and `hydra-bg`.

CI uses one workflow, `.github/workflows/ci.yaml`:

- A regular run does the unit tests, the Platform integration tests, and the Blockfrost tests.
- A nightly run also does the Hydra tests on the same commit.
- If `run_hydra` is `true`, a manual run also does the Hydra tests.

The `coverage_report` job sets the minimum line coverage for each type of run.
`LINE_THRESHOLD_REGULAR` is the minimum for a regular run.
`LINE_THRESHOLD_HYDRA` is the minimum for a run with the Hydra tests.

CI merges only the coverage artifacts from the current run.
CI does not use coverage from a different commit.
If a required test job does not pass, CI makes a diagnostic report without a threshold check.
The `coverage-report` artifact contains the HTML report. If the threshold check fails, CI also uploads this artifact.
Function coverage is only for information.
Different builds can give different symbol names to one function.
The report counts each function one time, at its source location.
The report does not include dependencies, build-only crates, test-only crates, or standalone test files.
