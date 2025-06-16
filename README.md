# Blockfrost Icebreakers API

The Blockfrost Icebreakers API provides a root endpoint to check the status and version of the API.

### Registration Process

When registering via the `/register` endpoint, the Blockfrost Icebreakers API (`icebreakers.blockfrost.io/api/`) performs the following checks:

- **Secret Verification:** Confirms that the provided secret is registered with Blockfrost.io.
- **NFT License Validation:** Ensures that the reward address contains an NFT issued by Blockfrost.io, which serves as a license.
- **Platform Accessibility Check:** Verifies that the platform is listening on the specified port and is publicly accessible.
- **User Data Storage:** Upon successful registration, the user's data is saved in the database.

### Configuration

```toml
[server]
address = '0.0.0.0:3000'
log_level = 'info'

[database]
connection_string = 'postgresql://user:pass@host:port/db'

[blockfrost]
project_id = 'BLOCKFROST_PROJECT_ID'
nft_asset = 'b0d07d45fe9514f80213f4020e5a61241458be626841cde717cb38a76e7574636f696e'
```

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

This app is hosted on the DigitalOcean App Platform. At the moment, the
following environments are deployed:

- https://api-dev.icebreakers.blockfrost.io/ from `master`

```cli
$ doctl apps create --spec=./do-dev.yml
Notice: App created
ID                                      Spec Name
Default Ingress    Active Deployment ID    In Progress Deployment ID    Created
At                                 Updated At
8877f0a6-f553-4a49-aa08-9683fbb4c610    blockfrost-icebreakers-api-dev
```

After that, you can view the logs.

```
$ doctl apps logs 8877f0a6-f553-4a49-aa08-9683fbb4c610
blockfrost-icebreakers-api 2024-08-20T18:48:18.346927157Z
blockfrost-icebreakers-api 2024-08-20T18:48:18.346977091Z Address:
🌍 http://0.0.0.0:3000
blockfrost-icebreakers-api 2024-08-20T18:48:18.346982280Z Log Level: 📘 INFO
```
