# blockfrost-icebreakers-api

![Blockfrost Logo](docs/logo.png)

When the blockfrost-instance starts, it needs to handshake with blockfrost.io to share some information that are required for both parties. The instance itself will first send the secret that the user got after registering with blockfrost.io, the reward address and the compact node. On the blockfrost.io icebreakers API (icebreakers.blockfrost.io/api/), we check if the secret is registrated, if the reward address contains the NFT issued by us (called licence) and if so, we generate a simple hash that will be served in the instance route (this is to prevent people to use the public instance).

#### Configuration

```toml
[server]
address = '0.0.0.0:3000'
log_level = 'info'

[database]
connection_string = 'postgresql://user:pass@host:port/db'

[blockfrost]
project_id = 'BLOCKFROST_PROJECT_ID'
nft_asset = 'b0d07d45fe9514f80213f4020e5a61241458be626841cde717cb38a76e7574636f696e'
api_url_pattern = 'https://{IP}:{PORT}/blockfrost/health'
```
