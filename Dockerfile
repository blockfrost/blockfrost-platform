FROM lukemathwalker/cargo-chef:latest-rust-1 AS base

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
  sccache=0.4.0~~pre6-1+b1 \
  pkg-config=1.8.1-1 \
  libssl-dev=3.0.16-1~deb12u1 \
  bzip2=1.0.8-5+b1 \
  tar=1.34+dfsg-1.2+deb12u1 \
  gzip=1.12.1 \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache
WORKDIR /app

FROM base AS planner
COPY ./src	./src
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
COPY ./src	./src
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
