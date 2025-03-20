# Building from the source code using nix

If you are using Nix, building `blockfrost-platform` is straightforward.

```bash
# To build the latest main version (experimental)
nix build github:blockfrost/blockfrost-platform

# To build a release version (recommended)
# NOTE: this option will be available after the first release
nix build github:blockfrost/blockfrost-platform/0.0.2
```

To make the builds much faster, it’s worth adding the IOG binary cache to your Nix configuration (`/etc/nix/nix.conf`):

```
substituters = https://cache.nixos.org https://cache.iog.io

trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ=
```

After the build is complete, you should see the binary file and can move on to the
Usage section of this documentation.

```bash
$ ./result/bin/blockfrost-platform --version
blockfrost-platform 0.0.2 (0000000000000000000000000000000000000000)
```
