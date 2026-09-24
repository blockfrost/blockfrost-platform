# 高度なオプション

Blockfrost プラットフォームは以下の高度なオプションを受け付けます。

`--server-address <SERVER_ADDRESS>`\
デフォルト: 0.0.0.0

`--server-port <SERVER_PORT>`\
デフォルト: 3000

`--server-concurrency-limit <LIMIT>`\
デフォルト: 8192\
サーバーが同時に処理する最大リクエスト数。この上限を超えるリクエストは 503 Service Unavailable で応答されます。

`--log-level <LOG_LEVEL>`\
デフォルト: info\
指定可能な値: debug, info, warn, error, trace

`--node-socket-path <CARDANO_NODE_SOCKET_PATH>` (必須)\
Cardano ノードソケットへのパス。ネットワークはノードから自動検出されます。

`--mode <MODE>`\
デフォルト: compact\
指定可能な値: compact, light, full

`--config <PATH>`\
既存の設定ファイルへのパス。

`--init`\
対話形式のウィザードで新しい設定ファイルを作成します。

`--solitary`\
Icebreakers API に登録せずソリタリーモードで実行します。\
`--secret` および `--reward-address` と競合します

`--secret <SECRET>`\
`--solitary` が指定されていない限り必須。\
`--solitary` と競合\
`--reward-address` を必要とします

`--reward-address <REWARD_ADDRESS>`\
`--solitary` が指定されていない限り必須。\
`--solitary` と競合\
`--secret` を必要とします

`--data-node <ENDPOINT>`\
チェーンデータの問い合わせに使用するデータノード (例: Dolos) の URL。

`--data-node-timeout-sec <SECONDS>`\
デフォルト: 30\
データノードリクエストのタイムアウト (秒)。

`--gateway-url <URL>`\
Gateway API の URL を上書き (デフォルト: ネットワークから導出)。セルフホストのゲートウェイやテストに有用です。

`--hydra-cardano-signing-key <PATH>`\
Hydra ヘッドの開閉時に L1 トランザクション手数料を支払うための、事前に資金を入れた Cardano 署名鍵へのパス (L2 ペイメントチャネルサイクルあたり約 13 ADA)。

`--no-metrics`\
Prometheus メトリクスエンドポイントを無効化します。

`--custom-genesis-config <PATH>`\
カスタム genesis 設定ファイルへのパス。

`--help`\
ヘルプ情報を表示

`--version`\
バージョン情報を表示
