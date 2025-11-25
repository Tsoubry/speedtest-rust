# Build
FROM docker.io/rust:1 as builder

WORKDIR /usr/local/src

COPY . .

RUN cargo build --release

# Run
FROM docker.io/debian:13

WORKDIR /usr/local/bin

RUN apt-get update && \
	apt-get install -y ca-certificates && \
	apt-get clean && \
	rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 appuser && chown -R 1000:1000 /usr/local/bin

USER 1000

COPY --from=builder \
	/usr/local/src/target/release/librespeed-rs \
	/bin/librespeed-rs

COPY configs.toml configs.toml

COPY assets assets

EXPOSE 8443

CMD ["/bin/librespeed-rs"]
