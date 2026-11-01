FROM docker.io/rust:1 as builder

WORKDIR /usr/local/src

COPY . .

# --locked ensures reproducible builds if Cargo.lock exists
RUN cargo build --release --locked


FROM docker.io/debian:13-slim

RUN useradd -m -u 1000 appuser

WORKDIR /usr/local/bin

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

RUN chown -R appuser:appuser /usr/local/bin

COPY --from=builder \
    /usr/local/src/target/release/librespeed-rs \
    /usr/local/bin/librespeed-rs

COPY --chown=appuser:appuser configs.toml configs.toml
COPY --chown=appuser:appuser assets assets

RUN chmod +x /usr/local/bin/librespeed-rs

USER 1000

EXPOSE 8080
EXPOSE 8443

CMD ["/usr/local/bin/librespeed-rs"]
