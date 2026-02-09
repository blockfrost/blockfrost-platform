# Blockfrost Platform Docs

http://platform.blockfrost.io

## Development

Before you start make sure you have downloaded and installed [Node.js LTS](https://nodejs.org/en/download/), [pnpm](https://pnpm.io/installation) and git.

1. install dependencies `pnpm install`
2. `pnpm dev`

## Production

Deployemnts are done by Vercel. Use UI to deploy new version.

## Rust coverage

Install [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) with `cargo install cargo-tarpaulin`.

Run `cargo tarpaulin --lib --out html`
