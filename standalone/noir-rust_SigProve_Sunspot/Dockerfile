
#nstalls nargo v1.0.0-beta.18 (circuit compiler)
FROM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        git \
        curl \
        bash \
        wget \
        tar \
    && rm -rf /var/lib/apt/lists/*

# Install Go 1.26.2 (required to build Sunspot)
RUN wget -q https://go.dev/dl/go1.26.2.linux-amd64.tar.gz \
    && tar -C /usr/local -xzf go1.26.2.linux-amd64.tar.gz \
    && rm go1.26.2.linux-amd64.tar.gz

ENV PATH="/usr/local/go/bin:${PATH}"
ENV GOPATH="/root/go-workspace"

RUN go version

# Build Sunspot at the commit that reads Nargo 1.0.0-beta.18 ACIR
RUN git clone https://github.com/reilabs/sunspot.git /tmp/sunspot \
    && git -C /tmp/sunspot checkout a58077e58366e0b2750b032652f2588a6776abb4 \
    && cd /tmp/sunspot/go \
    && go build -o /usr/local/bin/sunspot . \
    && rm -rf /tmp/sunspot

RUN sunspot --help

# Install nargo v1.0.0-beta.18 
RUN curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash \
    && /root/.nargo/bin/noirup --version 1.0.0-beta.18

ENV PATH="/root/.nargo/bin:${PATH}"

RUN nargo --version

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

# Copy binaries from builder 
COPY --from=builder /app/target/release/zk-rust-prototype    /usr/local/bin/api
COPY --from=builder /usr/local/bin/sunspot     /usr/local/bin/sunspot
COPY --from=builder /root/.nargo/bin/nargo     /usr/local/bin/nargo

# Copy Noir circuit source 
COPY proofsystem ./proofsystem

#Runtime directories 
RUN mkdir -p tmp keys

#Environment default
ENV PORT=4000
ENV WORKER_COUNT=4

EXPOSE 4000

CMD ["api"]
