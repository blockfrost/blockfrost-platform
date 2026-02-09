FROM lukemathwalker/cargo-chef:latest-rust-1 AS base

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

FROM gcr.io/distroless/cc-debian12:dca9008b864a381b5ce97196a4d8399ac3c2fa65 AS runtime
COPY --from=builder /app/target/release/blockfrost-platform /app/

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
