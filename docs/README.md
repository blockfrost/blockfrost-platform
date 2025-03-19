# Blockfrost Platform Docs

http://platform.blockfrost.io

## Development

Before you start make sure you have downloaded and installed [Node.js LTS](https://nodejs.org/en/download/), [Yarn](https://yarnpkg.com/lang/en/docs/install/) and git.

1. install dependecies `yarn`
2. `yarn dev`

## Production

Deployemnts are done by Vercel. Use UI to deploy new version.

## Rust coverage

Install [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) with `cargo install cargo-tarpaulin`.

Run `cargo tarpaulin --lib --out html`
