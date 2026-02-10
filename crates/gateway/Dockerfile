FROM rust:latest as builder

WORKDIR /usr/src/app

COPY . .

RUN rustup update --no-self-update stable && \
    rustup component add rustfmt rust-src clippy

RUN cargo build --release

FROM fedora:latest

RUN dnf install -y \
    libgcc \
    libstdc++ \
    postgresql-libs && \
    dnf clean all
WORKDIR /usr/src/app

COPY --from=builder /usr/src/app/target/release/blockfrost-gateway /usr/local/bin/blockfrost-gateway
COPY --from=builder /usr/src/app/config/development.toml /usr/local/bin/config/development.toml

ENTRYPOINT ["/usr/local/bin/blockfrost-gateway"]

CMD ["--config=/usr/local/bin/config/development.toml"]
