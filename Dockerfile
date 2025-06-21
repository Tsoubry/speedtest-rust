# Build
FROM docker.io/rust:1 as builder
WORKDIR /usr/local/src
COPY . .
RUN cargo build --release

# Run
FROM docker.io/debian:12
WORKDIR /usr/local/bin
COPY --from=builder \
	/usr/local/src/target/release/librespeed-rs \
	librespeed-rs
COPY configs.toml configs.toml
COPY assets assets
EXPOSE 8443
CMD ["librespeed-rs"]
