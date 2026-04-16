# Advanced options

The Blockfrost platform accepts the following advanced options:

`--server-address <SERVER_ADDRESS>`\
Default: 0.0.0.0

`--server-port <SERVER_PORT>`\
Default: 3000

`--server-concurrency-limit <LIMIT>`\
Default: 8192\
Maximum number of concurrent requests the server will handle. Requests exceeding this limit will receive a 503 Service Unavailable response.

`--log-level <LOG_LEVEL>`\
Default: info\
Possible values: debug, info, warn, error, trace

`--node-socket-path <CARDANO_NODE_SOCKET_PATH>` (required)\
Path to the Cardano node socket. The network is automatically detected from the node.

`--mode <MODE>`\
Default: compact\
Possible values: compact, light, full

`--config <PATH>`\
Path to an existing configuration file.

`--init`\
Initialize a new configuration file via an interactive wizard.

`--solitary`\
Run in solitary mode, without registering with the Icebreakers API.\
Conflicts with `--secret` and `--reward-address`

`--secret <SECRET>`\
Required unless `--solitary` is present.\
Conflicts with `--solitary`\
Requires `--reward-address`

`--reward-address <REWARD_ADDRESS>`\
Required unless `--solitary` is present.\
Conflicts with `--solitary`\
Requires `--secret`

`--data-node <ENDPOINT>`\
URL of a data node (e.g. Dolos) to use for querying chain data.

`--data-node-timeout-sec <SECONDS>`\
Default: 30\
Timeout in seconds for data node requests.

`--gateway-url <URL>`\
Override the Gateway API URL (default: derived from network). Useful for self-hosted gateways or testing.

`--hydra-cardano-signing-key <PATH>`\
Path to a prefunded Cardano signing key used to pay L1 transaction fees when opening and closing Hydra heads (roughly 13 ADA per L2 payment-channel cycle).

`--no-metrics`\
Disable the Prometheus metrics endpoint.

`--custom-genesis-config <PATH>`\
Path to a custom genesis configuration file.

`--help`\
Print help information

`--version`\
Print version information
