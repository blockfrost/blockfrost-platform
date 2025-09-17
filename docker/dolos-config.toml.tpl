[upstream]
peer_address = "PEER_ADDRESS"
network_magic = NETWORK_MAGIC
is_testnet = IS_TESTNET

[storage]
version = "v1"
path = "/data"
max_wal_history = 25920

[genesis]
byron_path = "/etc/genesis/NETWORK/byron.json"
shelley_path = "/etc/genesis/NETWORK/shelley.json"
alonzo_path = "/etc/genesis/NETWORK/alonzo.json"
conway_path = "/etc/genesis/NETWORK/conway.json"
force_protocol = 6

[sync]
pull_batch_size = 100

[submit]

[serve.grpc]
listen_address = "[::]:50051"
permissive_cors = true

[serve.ouroboros]
listen_path = "/dolos.socket"
magic = NETWORK_MAGIC

[serve.minibf]
listen_address = "[::]:3010"

[relay]
listen_address = "[::]:30031"
magic = NETWORK_MAGIC

[mithril]
aggregator = "MITHRIL_AGGREGATOR"
genesis_key = "MITHRIL_GENESIS_KEY"

[logging]
max_level = "INFO"
include_tokio = false
include_pallas = false
include_grpc = false
