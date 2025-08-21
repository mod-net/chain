###############
# Builder stage
###############
FROM rust:1-bookworm AS builder

ARG RUSTFLAGS
ENV CARGO_TERM_COLOR=always

# Build dependencies for Substrate
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    pkg-config \
    libssl-dev \
    cmake \
    protobuf-compiler \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Optionally leverage caching by copying manifests first
COPY Cargo.toml Cargo.lock ./
COPY node/Cargo.toml node/Cargo.toml
COPY runtime/Cargo.toml runtime/Cargo.toml
COPY pallets pallets

# Copy the full source
COPY . .

# Ensure wasm target available for substrate if needed
RUN rustup target add wasm32-unknown-unknown

# Build release binary
RUN cargo build --locked --release -p modnet-node

################
# Runtime stage
################
FROM debian:bookworm-slim AS runtime

ENV RUST_LOG=info \
    RUST_BACKTRACE=1

# Runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
  && rm -rf /var/lib/apt/lists/* \
  && update-ca-certificates

# Create non-root user and data dir
RUN useradd -m -u 10001 -U -s /usr/sbin/nologin modnet \
  && mkdir -p /data \
  && chown -R modnet:modnet /data

# Copy binary
COPY --from=builder /build/target/release/modnet-node /usr/local/bin/modnet-node

USER modnet
WORKDIR /data

EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/modnet-node"]
