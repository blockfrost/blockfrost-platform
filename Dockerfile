FROM lukemathwalker/cargo-chef:latest-rust-1-trixie AS base

# hadolint ignore=DL3008
RUN apt-get update \
  && apt-get install -y --no-install-recommends \
  sccache=0.10.0-4 \
  pkgconf=1.8.1-4 \
  libssl-dev \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache
WORKDIR /app

FROM base AS planner
COPY ./crates	./crates
COPY Cargo.toml	Cargo.lock	./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
  ls -l ; cargo chef prepare --recipe-path recipe.json

FROM base AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
  cargo chef cook --release --workspace --recipe-path recipe.json
COPY ./crates	./crates
COPY Cargo.toml	Cargo.lock	./
ARG GIT_REVISION
ENV GIT_REVISION=$GIT_REVISION
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
  cargo build --release

FROM base AS hydra
ARG TARGETARCH
# hadolint ignore=DL3008
RUN apt-get update \
  && apt-get install -y --no-install-recommends unzip patchelf \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*
RUN set -eux; \
  mkdir -p /app/hydra-node; \
  if [ "$TARGETARCH" = "amd64" ]; then \
    curl -fSL -o /tmp/hydra.zip \
      "https://github.com/cardano-scaling/hydra/releases/download/1.0.0/hydra-x86_64-linux-1.0.0.zip" \
    && unzip /tmp/hydra.zip -d /app/hydra-node \
    && rm /tmp/hydra.zip; \
  elif [ "$TARGETARCH" = "arm64" ]; then \
    curl -fSL -o /tmp/hydra.tar.bz2 \
      "https://github.com/blockfrost/hydra-aarch64-linux/releases/download/1.0.0/hydra-aarch64-linux-1.0.0.tar.bz2" \
    && tar xjf /tmp/hydra.tar.bz2 -C /app/hydra-node \
    && rm /tmp/hydra.tar.bz2 \
    && patchelf \
      --set-interpreter /app/hydra-node/lib/ld-linux-aarch64.so.1 \
      --set-rpath /app/hydra-node/lib \
      /app/hydra-node/exe/hydra-node \
    && rm -f /app/hydra-node/hydra-node \
    && mv /app/hydra-node/exe/hydra-node /app/hydra-node/hydra-node \
    && rm -rf /app/hydra-node/bin /app/hydra-node/exe; \
  else \
    echo "Unsupported architecture: $TARGETARCH" >&2; exit 1; \
  fi; \
  chmod +x /app/hydra-node/hydra-node; \
  /app/hydra-node/hydra-node --version

FROM gcr.io/distroless/cc-debian13@sha256:05d26fe67a875592cd65f26b2bcfadb8830eae53e68945784e39b23e62c382e0 AS runtime
COPY --from=builder /app/target/release/blockfrost-platform /app/
COPY --from=hydra /app/hydra-node/ /app/hydra-node/

RUN ["/app/blockfrost-platform", "--version"]

ARG GIT_REVISION
LABEL org.opencontainers.image.title="Blockfrost platform" \
  org.opencontainers.image.url="https://platform.blockfrost.io/" \
  org.opencontainers.image.description="The Blockfrost platform transforms your Cardano node infrastructure into a high-performance JSON API endpoint." \
  org.opencontainers.image.licenses="Apache-2.0" \
  org.opencontainers.image.source="https://github.com/blockfrost/blockfrost-platform" \
  org.opencontainers.image.revision=$GIT_REVISION

EXPOSE 3000/tcp
STOPSIGNAL SIGINT
ENTRYPOINT ["/app/blockfrost-platform"]
