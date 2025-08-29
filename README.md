# Blockfrost Gateway

The Blockfrost Gateway provides a root endpoint to check the status and version of the API.

### Registration Process

When registering via the `/register` endpoint, the Blockfrost Gateway performs the following checks:

- **Secret Verification:** Confirms that the provided secret is registered with Blockfrost.io.
- **NFT License Validation:** Ensures that the reward address contains an NFT issued by Blockfrost.io, which serves as a license.
- **Platform Accessibility Check:** Verifies that the platform is listening on the specified port and is publicly accessible.
- **User Data Storage:** Upon successful registration, the user's data is saved in the database.

### Configuration

```toml
[server]
address = '0.0.0.0:3001'
log_level = 'info'

[database]
connection_string = 'postgresql://user:pass@host:port/db'

[blockfrost]
project_id = 'BLOCKFROST_PROJECT_ID'
nft_asset = 'b0d07d45fe9514f80213f4020e5a61241458be626841cde717cb38a76e7574636f696e'

```

#### Environment Variables

The following environment variables can be used to override config file:

- `SERVER_ADDRESS` — The server address (e.g., `0.0.0.0:3000`)
- `SERVER_LOG_LEVEL` — The log level (e.g., `info`, `debug`, `warn`)
- `DB_CONNECTION_STRING` — The database connection string (PostgreSQL supported)
- `BLOCKFROST_PROJECT_ID` — The Blockfrost project ID
- `BLOCKFROST_NFT_ASSET` — Hex of the NFT asset used for validating license


### Development

This repository has a [devshell](https://github.com/numtide/devshell) configured for Linux and macOS machines (both x86-64 and AArch64). To use it, please install [Nix](https://nixos.org/download/), [direnv](https://direnv.net/), enter the cloned directory, and run `direnv allow`

To run dev server:

```
cargo run -- --config="./config/development.toml"
```

If you are not using nix and you are getting an error `ld: library 'pq' not found`, on MacOS you need to install `libpq` and `diesel_cli`:

```
brew install libpq && brew link --force libpq
cargo clean
cargo install diesel_cli --no-default-features --features postgres
```

## DigitalOcean

To quickly host this on the DigitalOcean App Platform change `domains.domain` in `do-dev.yml` and run:

```cli
$ doctl apps create --spec=./do-dev.yml
Notice: App created
ID                                      Spec Name
Default Ingress    Active Deployment ID    In Progress Deployment ID    Created
At                                 Updated At
8877f0a6-f553-4a49-aa08-9683fbb4c610    blockfrost-gateway-dev
```

After that, you can view the logs.

```
$ doctl apps logs 8877f0a6-f553-4a49-aa08-9683fbb4c610
blockfrost-gateway 2024-08-20T18:48:18.346927157Z
blockfrost-gateway 2024-08-20T18:48:18.346977091Z Address:
🌍 http://0.0.0.0:3000
blockfrost-gateway 2024-08-20T18:48:18.346982280Z Log Level: 📘 INFO
```
