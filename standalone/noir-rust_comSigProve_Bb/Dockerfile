FROM rust:1-bookworm AS builder

WORKDIR /app

ARG NARGO_VERSION=1.0.0-beta.19
ARG BB_VERSION=3.0.0-nightly.20260102

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       pkg-config \
       clang \
       cmake \
       curl \
    && rm -rf /var/lib/apt/lists/*

# Install Nargo
RUN curl -fsSL "https://github.com/noir-lang/noir/releases/download/v${NARGO_VERSION}/nargo-x86_64-unknown-linux-gnu.tar.gz" \
    | tar -xz -C /usr/local/bin

# Install Barretenberg
RUN curl -fsSL "https://github.com/AztecProtocol/barretenberg/releases/download/v${BB_VERSION}/barretenberg-amd64-linux.tar.gz" -o /tmp/bb.tar.gz \
    || curl -fsSL "https://github.com/AztecProtocol/aztec-packages/releases/download/v${BB_VERSION}/barretenberg-amd64-linux.tar.gz" -o /tmp/bb.tar.gz \
    && tar -xzf /tmp/bb.tar.gz -C /usr/local/bin \
    && rm /tmp/bb.tar.gz

# Copy source files
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY proofsystem ./proofsystem

# Build Rust application
RUN cargo build --release

# Compile Noir circuit
RUN cd proofsystem && rm -rf target && nargo compile

# ------------------------------------------------------

FROM ubuntu:24.04 AS runtime

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       ca-certificates \
       git \
       libstdc++6 \
       libgomp1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/native-rust-prototype /usr/local/bin/native-rust-prototype
COPY --from=builder /usr/local/bin/nargo /usr/local/bin/nargo
COPY --from=builder /usr/local/bin/bb /usr/local/bin/bb
COPY --from=builder /app/proofsystem ./proofsystem

RUN mkdir -p /app/tmp /app/keys

ENV PORT=4000
ENV WORKER_COUNT=4

EXPOSE 4000

CMD ["native-rust-prototype"]