FROM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        git \
        curl \
        wget \
        tar \
    && rm -rf /var/lib/apt/lists/*

RUN wget -q https://go.dev/dl/go1.26.2.linux-amd64.tar.gz \
    && tar -C /usr/local -xzf go1.26.2.linux-amd64.tar.gz \
    && rm go1.26.2.linux-amd64.tar.gz

ENV PATH="/usr/local/go/bin:${PATH}"

RUN git clone https://github.com/reilabs/sunspot.git /tmp/sunspot \
    && git -C /tmp/sunspot checkout a58077e58366e0b2750b032652f2588a6776abb4 \
    && cd /tmp/sunspot/go \
    && go build -o /usr/local/bin/sunspot . \
    && rm -rf /tmp/sunspot

RUN curl -fsSL \
    "https://github.com/noir-lang/noir/releases/download/v1.0.0-beta.18/nargo-x86_64-unknown-linux-gnu.tar.gz" \
    | tar -xz -C /usr/local/bin/ \
    && chmod +x /usr/local/bin/nargo

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo 'fn main(){}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
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

COPY --from=builder /app/target/release/proofsystem /usr/local/bin/proofsystem
COPY --from=builder /usr/local/bin/nargo /usr/local/bin/nargo
COPY --from=builder /usr/local/bin/sunspot /usr/local/bin/sunspot
COPY proofsystem ./proofsystem

RUN chmod +x /usr/local/bin/sunspot \
    && mkdir -p tmp keys \
    && chmod 777 tmp keys

ENV PORT=4000
ENV WORKER_COUNT=4

EXPOSE 4000

CMD ["proofsystem"]
