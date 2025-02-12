### Building the Docker image

To build the Docker image containing the project binary, in the root folder of the repository:

```console
# Clone the repository
git clone https://github.com/blockfrost/blockfrost-platform

# Navigate to the project directory
cd blockfrost-platform

# To build the latest main version (experimental)
git checkout main

# To build a release version (recommended)
# NOTE: this option will be available after the first release
# git checkout v0.1

# Build the docker image
docker build -t blockfrost-platform .
```

Or you can simply pull it directly from GitHub:

```console
# Pulling the latest build (experimental)
docker pull ghcr.io/blockfrost/blockfrost-platform

# Pulling a specific version (recommended)
# NOTE: this option will be available after the first release
# docker pull ghcr.io/blockfrost/blockfrost-platform:v0.1
```

After you have your Docker image on your machine, you can proceed with the "Usage -> Docker" section of this documentation.
