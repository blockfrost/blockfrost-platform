# Nix を使ってバイナリをビルドする

Nix を使用している場合、`blockfrost-platform` のビルドは非常に簡単です。

```bash
# 最新の main 版 (実験的) をビルドする場合
nix build github:blockfrost/blockfrost-platform

# リリース版 (推奨) をビルドする場合
nix build github:blockfrost/blockfrost-platform/1.0.0
```

ビルドを大幅に高速化するには、Nix 設定 (`/etc/nix/nix.conf`) に IOG バイナリキャッシュを追加することをおすすめします。

```
substituters = https://cache.nixos.org https://cache.iog.io

trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ=
```

ビルドが完了するとバイナリファイルが生成されます。
その後 [プラットフォームの設定](/configuration) に進めます。

```bash
$ ./result/bin/blockfrost-platform --version
blockfrost-platform 1.0.0 (<1134f1a3027e3cbaf81f5e5595aadd1bcfefdae7>)
```
