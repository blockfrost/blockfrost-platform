# Building the Docker image

To build the Docker image containing the project binary, in the root folder of the repository:

```console
# Clone the repository
git clone https://github.com/blockfrost/blockfrost-platform

# Navigate to the project directory
cd blockfrost-platform

# To build the latest main version (experimental)
git checkout main

# To build a release version (recommended)
git checkout 0.0.2

# Build the docker image
docker build -t blockfrost-platform .
```

Or you can simply pull it directly from GitHub:

```console
# Pulling the latest build (experimental)
docker pull ghcr.io/blockfrost/blockfrost-platform:edge

# Pulling the latest release (recommended)
docker pull ghcr.io/blockfrost/blockfrost-platform:latest

# Pulling a specific version
docker pull ghcr.io/blockfrost/blockfrost-platform:0.0.2
```

After you have your Docker image on your machine, you can move on to [Configuring the platform](/configuration).
