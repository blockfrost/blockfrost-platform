# Verifying that you are up and running

## Prerequisites verification

Before verifying your Blockfrost platform operation, make sure that these critical configurations are correct:

### A running Cardano node

A running mainnet (production) instance of cardano-node with a public IP address.

### NFT address configuration

**Important:** You must use the exact address containing your Icebreaker NFT, not your general wallet address.

When running the init command or configuring manually, use the specific address containing the NFT:

```shell
--reward-address addr1xxx...  # Address containing NFT, not general wallet address
```

Your wallet likely contains multiple addresses.
The platform requires specifically the address where the NFT resides.
Check your wallet to identify which address holds the Icebreaker NFT.

### Network port configuration

Make sure that your required ports are properly opened.
The default Blockfrost platform port is `3000`.

#### For cloud hosting

- Azure: Create a new outbound rule allowing port 3000 traffic

- AWS: Configure security groups to permit port 3000 traffic

- Other providers: Update firewall settings accordingly.

## Basic service verification

After confirming prerequisites, verify the service itself:

### Installation verification

Confirm that the software is properly installed:

```shell
blockfrost-platform --version
```

This should return the current version number.

### Service startup verification

Start the service and check for successful initialization:

```shell
blockfrost-platform --network mainnet \
                    --node-address 127.0.0.1 \
                    --secret your_icebreaker_secret \
                    --reward-address your_nft_address
```

In the logs, you should see initialization messages.
The message `DEBUG: Decoding done` indicates normal operation.

If you don't see this message or encounter errors, check:

- Node socket path accessibility
- Network configuration
- Secret key validity.

### Operational verification

Once the service is running, verify that it is functioning correctly:

### Understanding the UUID

The Blockfrost platform generates a new UUID each time it starts.
This UUID is critical for accessing endpoints:

### Example log entry showing UUID

```
INFO: Your instance ID: 3fa85f64-5717-4562-b3fc-2c963f66afa6
                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

You would need this UUID for things like your own manual transaction submission tests.
Otherwise, it's an implementation detail.
Remember that this changes every time you restart the service.

### Metrics endpoint verification

Check if your service is properly reporting metrics:

```shell
wget http://your-server-ip:3000/metrics
```

If functioning properly, this endpoint provides:

- API calls served count
- Resource utilization statistics
- Uptime information.

Note: The metrics endpoint may require the latest binary.
If you receive a 500 error, update your installation.

### Health endpoint verification

Check if your service is up and running by calling the health endpoint:

```shell
wget http://your-server-ip:3000/
```

If working properly, this endpoint will return JSON with service status details such as:

- Application name
- Current release version
- Revision (Git commit hash)
- Boolean indicating overall health (`healthy`)
- Block, epoch, era, slot, and sync progress info
- Array listing any issues detected

If `healthy` is true, the service is operational.
If you get a 503 error or if `healthy` is false, check your logs or update your installation to resolve potential issues.

## Icebreaker dashboard monitoring

For a visual verification of your node's status, access the Icebreaker dashboard at [http://icebreakers.blockfrost.io/](http://icebreakers.blockfrost.io/)

Your node will appear as 'Icebreaker X' where X is your assigned number (visible in your NFT metadata).

This dashboard provides:

- Connection status
- Performance metrics
- Comparison with other Icebreaker nodes.

## Troubleshooting common issues

If verification fails, check these common issues:

### Not seeing 'Decoding done' message

- Ensure your Cardano node is fully synced
- Check node socket path accessibility.

### Cannot access metrics endpoint

- Verify you're using the current UUID
- Confirm port 3000 is accessible from your network
- Check if you need to update to the latest binary.

### Not appearing in Grafana

- Allow up to 10 minutes for your node to appear
- Verify your NFT address is correctly configured
- Ensure outbound connections are permitted by your firewall.
