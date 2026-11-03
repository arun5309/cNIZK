FROM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        git \
    && rm -rf /var/lib/apt/lists/*

#  Install provekit-cli v0.1.4 (compiles the circuit itself; nargo is not needed)
RUN cargo install --locked provekit-cli --version 0.1.4

# ── Cache Rust dependencies before copying source 
# Copy manifests first so Docker layer-caches the dep build
COPY Cargo.toml Cargo.lock ./
# Dummy main so `cargo build` resolves deps without full source
RUN mkdir -p src && echo 'fn main(){}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Copy real source and build 
COPY src ./src
# Touch main.rs so cargo knows it changed (avoids stale cache)
RUN touch src/main.rs && cargo build --release


FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        git \
        libssl3 \
        libstdc++6 \
        libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries from builder 
COPY --from=builder /app/target/release/api          /usr/local/bin/api
COPY --from=builder /usr/local/cargo/bin/provekit-cli /usr/local/bin/provekit-cli

# Copy Noir circuit source 
# provekit-cli compile needs the full proofsystem/ directory at runtime
COPY proofsystem ./proofsystem

# Runtime directories
RUN mkdir -p tmp keys

# Environment defaults (override via docker-compose / -e flags) ─
ENV PORT=4000
ENV WORKER_COUNT=4

EXPOSE 4000

CMD ["api"]