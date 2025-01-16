# Advanced options

The Blockfrost platform accepts the following advanced options:

`--server-address <SERVER_ADDRESS>`
Default: 0.0.0.0

`--server-port <SERVER_PORT>`
Default: 3000

`--network <NETWORK> (required)`
Possible values: mainnet, preprod, preview, sanchonet

`--log-level <LOG_LEVEL>`
Default: info
Possible values: debug, info, warn, error, trace

`--node-socket-path <NODE_SOCKET_PATH> (required)`

`--mode <MODE>`
Default: compact
Possible values: compact, light, full

`--solitary`
Run in solitary mode, without registering with the Icebreakers API
Conflicts with --secret and --reward-address

`--secret <SECRET>`
Required unless --solitary is present
Conflicts with --solitary
Requires --reward-address

`--reward-address <REWARD_ADDRESS>`
Required unless --solitary is present
Conflicts with --solitary
Requires --secret

`--help`
Print help information

`--version`
Print version information
