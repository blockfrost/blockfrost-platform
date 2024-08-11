# Blockfrost Icebreakers API

The Blockfrost Icebreakers API provides a root endpoint to check the status and version of the API.

### Registration Process

When registering via the `/register` endpoint, the Blockfrost Icebreakers API (`icebreakers.blockfrost.io/api/`) performs the following checks:

- **Secret Verification:** Confirms that the provided secret is registered with Blockfrost.io.
- **NFT License Validation:** Ensures that the reward address contains an NFT issued by Blockfrost.io, which serves as a license.
- **Hash Generation:** If the secret and NFT license are successfully verified, the API generates a unique hash. This hash is then made available at a specific route, allowing only authorized users to access the instance, thereby preventing misuse of the public instance.
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
api_url_pattern = 'https://{IP}:{PORT}/blockfrost/health'
```
