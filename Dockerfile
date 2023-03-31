FROM rust:1.68-slim AS builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    RUSTFLAGS='-C target-feature=+crt-static' \
        cargo build --release --target x86_64-unknown-linux-gnu


FROM scratch

COPY --from=builder /build/target/x86_64-unknown-linux-gnu/release/catchall-api /catchall

EXPOSE 8080

ENTRYPOINT ["/catchall"]
