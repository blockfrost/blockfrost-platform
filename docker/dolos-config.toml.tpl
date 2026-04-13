[chain]
is_testnet = IS_TESTNET
magic = NETWORK_MAGIC
type = "cardano"

[genesis]
alonzo_path = "GENESIS_PREFIX/NETWORK/alonzo.json"
byron_path = "GENESIS_PREFIX/NETWORK/byron.json"
conway_path = "GENESIS_PREFIX/NETWORK/conway.json"
FORCE_PROTOCOL_LINE
shelley_path = "GENESIS_PREFIX/NETWORK/shelley.json"

[logging]
include_grpc = false
include_pallas = false
include_tokio = false
include_trp = false
max_level = "INFO"

[mithril]
aggregator = "MITHRIL_AGGREGATOR"
ancillary_key = "MITHRIL_ANCILLARY_KEY"
genesis_key = "MITHRIL_GENESIS_KEY"

[serve.minibf]
listen_address = "[::]:3010"
token_registry_url = "TOKEN_REGISTRY_URL"

[storage]
max_wal_history = 25920
path = "STORAGE_PATH"
version = "v3"

[submit]

[sync]
pull_batch_size = 100

[upstream]
peer_address = "PEER_ADDRESS"
