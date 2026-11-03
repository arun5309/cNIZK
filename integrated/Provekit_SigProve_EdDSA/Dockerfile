FROM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        git \
        curl \
        bash \
    && rm -rf /var/lib/apt/lists/*

RUN curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash \
    && /root/.nargo/bin/noirup --version 1.0.0-beta.11

ENV PATH="/root/.nargo/bin:${PATH}"

RUN nargo --version
RUN cargo install --locked provekit-cli --version 0.1.4

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        git \
        libssl3 \
        libstdc++6 \
        libgomp1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/api /usr/local/bin/api
COPY --from=builder /usr/local/cargo/bin/provekit-cli /usr/local/bin/provekit-cli
COPY --from=builder /root/.nargo/bin/nargo /usr/local/bin/nargo
COPY proofsystem ./proofsystem

RUN mkdir -p tmp keys && chmod 777 tmp keys

ENV PORT=4001
ENV WORKER_COUNT=4

EXPOSE 4001

CMD ["api"]
