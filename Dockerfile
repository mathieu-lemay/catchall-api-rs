FROM rust:1.68-slim AS builder

RUN set -ex; \
    mkdir -p -m 0600 ~/.ssh; \
    apt update && apt install -y --no-install-recommends openssh-client; \
    ssh-keyscan -p 10022 gitea.acidrain.duckdns.org >> ~/.ssh/known_hosts;

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
