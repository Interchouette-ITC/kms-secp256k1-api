# Latest Debian (13 / trixie) builder — not bookworm.
FROM rust:slim-trixie AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
  pkg-config \
  libssl-dev \
  ca-certificates \
  build-essential \
  curl \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to allow building deps
RUN mkdir src && echo "fn main() {}" > src/main.rs

RUN cargo fetch

RUN cargo build --release

# Remove dummy sources
RUN rm -rf src

COPY src ./src

COPY wasm ./wasm

RUN cargo build --release

# Distroless on Debian 13 (trixie family): glibc + libgcc/libstdc++, no shell/apt/perl.
# Binary is rustls-only (ldd: libc, libm, libgcc) — no OpenSSL .so required.
FROM gcr.io/distroless/cc-debian13:nonroot AS runtime

WORKDIR /app

# VERSION is read from Cargo.toml at runtime
COPY --from=builder /app/Cargo.toml /app/Cargo.toml
COPY --from=builder /app/target/release/kms-secp256k1-api /app/kms-secp256k1-api
COPY --from=builder /app/wasm /app/wasm
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV APP_ADDR=0.0.0.0
ENV APP_PORT=4000

EXPOSE 4000

USER nonroot:nonroot

CMD ["/app/kms-secp256k1-api"]
