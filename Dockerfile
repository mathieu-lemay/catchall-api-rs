FROM rust:1.68-slim AS builder

# DL3008: Pin versions in apt get install
# hadolint ignore=DL3008
RUN set -ex; \
    mkdir -m 0600 ~/.ssh; \
    apt-get update && apt-get install -y --no-install-recommends openssh-client; \
    echo '[gitea.acidrain.duckdns.org]:10022 ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBGH0muaZTfyWOUDLS4kWJtaSISMfAU87zsf68FDLVAYeiPwMjGrYvZX51tXSeeeI3A29LyiPlKsMUuhih/EMA8w=' > ~/.ssh/known_hosts;

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=ssh \
    RUSTFLAGS='-C target-feature=+crt-static' \
        cargo build --release --target x86_64-unknown-linux-gnu


FROM scratch

COPY --from=builder /build/target/x86_64-unknown-linux-gnu/release/catchall-api /catchall

EXPOSE 8080

ENTRYPOINT ["/catchall"]
