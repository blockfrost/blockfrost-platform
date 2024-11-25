# Blockfrost Platform

`blockfrost-platform` is a software that serves the Blockfrost API within the Blockfrost network. It is designed to decentralize Blockfrost by allowing Stake Pool Operators (SPOs) and other node operators to run Blockfrost instances, which will handle customer traffic.

## Key Features

- **Decentralization:** Enables the decentralization of Blockfrost services by distributing traffic across multiple instances.
- **SPO Engagement:** Involves SPOs and other node operators in the process, enhancing the robustness of the network.
- **Rust-based CLI:** Built with Rust, offering performance and security.

## Prerequisites

- A running Cardano node with a socket available.
- Rust environment for building from source (if not using pre-built binaries).

## Installation

To install `blockfrost-platform`, you can download the pre-built binaries from the [releases page](#) or build from source:

```bash
# Clone the repository
git clone https://github.com/blockfrost/blockfrost-platform

# Navigate to the project directory
cd blockfrost-platform

# Build the project
cargo build --release

```

## Usage

```shell
blockfrost-platform [OPTIONS] --network <NETWORK> --node-address <NODE_ADDRESS> --secret <SECRET> --reward-address <REWARD_ADDRESS>
```

### Options

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

## Devshell

This repository has a [devshell](https://github.com/numtide/devshell) configured for Linux and macOS machines, both x86-64, and AArch64. To use it, please:

1. Install:
   - [Nix](https://nixos.org/download/),
   - [direnv](https://direnv.net/),
   - optionally: [nix-direnv](https://github.com/nix-community/nix-direnv) for a slightly better performance, if it’s easy for you to enable, e.g. on NixOS, [nix-darwin](https://github.com/LnL7/nix-darwin), using [home-manager](https://github.com/nix-community/home-manager) etc.
2. Enter the cloned directory.
3. And run `direnv allow`.

### Pure Nix builds

You can also use `nix build` to build the package for these platforms.

If in doubt, run `nix flake show`.

## Docker

### Building & running blockfrost-platform

To build the Docker image containing the project binary, in the root folder of the repository:

```console
docker build -t blockfrost-platform .
```

The Docker image named `blockfrost-platform` is locally available. To run it:

```console
docker run -it --init --rm \
-p 3000:3000 \
-v /home/user/my_node.socket:/var/run/node.socket \
blockfrost-platform --node-socket-path /var/run/node.socket \
--network preview --secret my_secret --reward-address my_reward_address
```

> **_NOTE:_** Make sure your Cardano node socket is attached as a volume, i.e. `-v /home/user/my_node.socket:/var/run/node.socket`.

> **_NOTE:_** If you don't specify an IP address (i.e., `-p 3000:3000` instead of `-p 127.0.0.1:3000:3000`) when publishing a container's ports, Docker publishes the port on all interfaces (address `0.0.0.0`) by default. These ports are externally accessible.

### Building & running blockfrost-platform and required services

The below command will build and run the `blocfrost-platform` binary, along with the Cardano node, using Docker Compose.

In the root folder of the repository:

```console
# Solitary Mode
NETWORK=preview docker compose -p preview --profile solitary up --build -d
# Or use it with blockfrost-icebreakers-api
NETWORK=preview SECRET=my-secret REWARD_ADDRESS=my-reward-address docker compose -p preview up --build -d
docker compose watch # Auto rebuild on changes
```

> **_NOTE:_** If you want to avoid running it in the background, omit the `-d` flag.

> **_NOTE:_** If you want to skip building, omit the `--build` flag.

> **_NOTE:_** Setting `-p preview` to the desired network will let you run on different networks without messing your node db. You can omit it if you plan to run on the same network always.

> **_NOTE:_** You don't need to provide `--node-socket-path` since it is already handled inside `docker-compose.yml`.
