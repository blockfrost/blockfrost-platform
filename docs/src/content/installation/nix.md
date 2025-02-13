## Building from the source code using nix

If you are using Nix, building `blockfrost-platform` is straightforward.

```bash
# Clone the repository
git clone https://github.com/blockfrost/blockfrost-platform

# Navigate to the project directory
cd blockfrost-platform

# To build the latest main version (experimental)
git checkout main

# To build a release version (recommended)
# NOTE: this option will be available after the first release
# git checkout v0.1

# Build the project using nix
nix build
```

After the build is complete, you should see the binary file and can move on to the
Usage section of this documentation.

```bash
$ ./result/bin/blockfrost-platform --version
blockfrost-platform 0.0.1
```
