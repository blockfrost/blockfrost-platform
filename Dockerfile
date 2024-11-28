FROM lukemathwalker/cargo-chef:latest-rust-slim-bookworm AS base
RUN apt update ; apt install sccache pkg-config libssl-dev bzip2 -y
ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache
WORKDIR /app

FROM base AS planner
COPY ./src	./src
COPY Cargo.toml	Cargo.lock	./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    ls -l ; cargo chef prepare --recipe-path recipe.json

FROM base AS downloader
ADD https://github.com/input-output-hk/testgen-hs/releases/download/10.1.2.1/testgen-hs-10.1.2.1-x86_64-linux.tar.bz2 /app/
RUN tar -xjf testgen-hs-*.tar.* && /app/testgen-hs/testgen-hs --version

FROM base AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef cook --release --workspace --recipe-path recipe.json
COPY ./src	./src
COPY Cargo.toml	Cargo.lock	./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo build --release

FROM gcr.io/distroless/cc-debian12	as runtime
COPY --from=builder /app/target/release/blockfrost-platform /app/
COPY --from=downloader /app/testgen-hs /app/testgen-hs
EXPOSE 3000/tcp
STOPSIGNAL SIGINT
ENTRYPOINT ["/app/blockfrost-platform"]
