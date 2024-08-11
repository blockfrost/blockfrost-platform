# Blockfrost Icebreakers API

<img src="docs/logo.png" alt="Blockfrost Logo" width="150"/>

When a Blockfrost instance starts, it needs to perform an initial handshake with Blockfrost.io to exchange crucial information required by both parties. This handshake involves the instance sending the following details:

1. **User Secret:** The secret key provided to the user during registration with Blockfrost.io.
2. **Reward Address:** The address where the user will receive rewards.
3. **Compact Node:** The node used by the instance.

Upon receiving this information, the Blockfrost Icebreakers API (`icebreakers.blockfrost.io/api/`) performs several checks:

- **Secret Verification:** Confirms that the provided secret is registered with Blockfrost.io.
- **NFT License Validation:** Checks whether the reward address contains the NFT issued by Blockfrost.io, which acts as a license.
- **Hash Generation:** If both verifications are successful, the API generates a unique hash. This hash is then served by the instance in a specific route, ensuring that only authorized users can access the instance, preventing misuse of the public instance.

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
