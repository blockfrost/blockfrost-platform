FROM	rust:1.82	as	builder
WORKDIR	/usr/src
COPY	Cargo.toml	Cargo.lock	./
RUN	mkdir src \
	&& touch src/lib.rs
RUN	cargo build --release \
	&& rm -rf src
COPY	./src	./src
RUN	touch ./src/main.rs ./src/lib.rs # trigger a rebuild
RUN	cargo build --release
FROM	gcr.io/distroless/cc-debian12	as	runtime
COPY	--from=builder /usr/src/target/release/blockfrost-platform	/bin
EXPOSE	3000/tcp
STOPSIGNAL	SIGINT
ENTRYPOINT	["blockfrost-platform"]
