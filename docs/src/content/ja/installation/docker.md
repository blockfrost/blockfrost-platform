# Docker イメージをビルドする

プロジェクトバイナリを含む Docker イメージをビルドするには、リポジトリのルートフォルダで:

```console
# リポジトリをクローン
git clone https://github.com/blockfrost/blockfrost-platform

# プロジェクトディレクトリへ移動
cd blockfrost-platform

# 最新の main 版 (実験的) をビルドする場合
git checkout main

# リリース版 (推奨) をビルドする場合
git checkout 1.0.0

# Docker イメージをビルド
docker build -t blockfrost-platform .
```

または、GitHub から直接 pull することもできます。

```console
# 最新ビルドを pull (実験的)
docker pull ghcr.io/blockfrost/blockfrost-platform:edge

# 最新リリースを pull (推奨)
docker pull ghcr.io/blockfrost/blockfrost-platform:latest

# 特定のバージョンを pull
docker pull ghcr.io/blockfrost/blockfrost-platform:1.0.0
```

Docker イメージが手元に揃ったら、[プラットフォームの設定](/configuration) に進めます。
