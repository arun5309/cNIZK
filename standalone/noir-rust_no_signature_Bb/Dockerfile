FROM rust:1-bookworm AS builder

WORKDIR /app

ARG NARGO_VERSION=1.0.0-beta.19
ARG BB_VERSION=3.0.0-nightly.20260102

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config clang cmake curl \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL "https://github.com/noir-lang/noir/releases/download/v${NARGO_VERSION}/nargo-x86_64-unknown-linux-gnu.tar.gz" \
    | tar -xz -C /usr/local/bin

RUN curl -fsSL "https://github.com/AztecProtocol/barretenberg/releases/download/v${BB_VERSION}/barretenberg-amd64-linux.tar.gz" -o /tmp/bb.tar.gz \
    || curl -fsSL "https://github.com/AztecProtocol/aztec-packages/releases/download/v${BB_VERSION}/barretenberg-amd64-linux.tar.gz" -o /tmp/bb.tar.gz \
    && tar -xz -C /usr/local/bin -f /tmp/bb.tar.gz \
    && rm /tmp/bb.tar.gz

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

COPY proofsystem ./proofsystem
RUN cd proofsystem && rm -rf target && nargo compile

FROM ubuntu:24.04 AS runtime

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git libstdc++6 libgomp1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/proofsystem /usr/local/bin/proofsystem
COPY --from=builder /usr/local/bin/nargo /usr/local/bin/nargo
COPY --from=builder /usr/local/bin/bb /usr/local/bin/bb
COPY --from=builder /app/proofsystem ./proofsystem

RUN mkdir -p tmp vk

ENV PORT=4000
ENV WORKER_COUNT=4

EXPOSE 4000

CMD ["proofsystem"]
