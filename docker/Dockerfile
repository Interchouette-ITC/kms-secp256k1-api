FROM rust:slim-trixie AS builder

RUN apt-get update && apt-get install -y \
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

FROM debian:trixie-slim AS runtime

RUN apt-get update && \
  apt-get upgrade -y && \
  apt-get install -y ca-certificates && \
  rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/Cargo.toml .
COPY --from=builder /app/target/release/kms-secp256k1-api .
COPY --from=builder /app/wasm ./wasm


EXPOSE ${APP_PORT}

CMD ["./kms-secp256k1-api"]
