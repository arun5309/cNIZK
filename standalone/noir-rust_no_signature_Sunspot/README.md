# Noir ZK Proof Service (No Signature) — Sunspot

This service proves that 5 data values satisfy 5 constraints. There is no signature. The circuit does not hash the data and does not check a public key.

Built with Rust (Axum) + Noir + Sunspot.

## Tool versions

| Tool | Version |
|---|---|
| Nargo / Noir | `1.0.0-beta.18` |
| Sunspot | commit `a58077e58366e0b2750b032652f2588a6776abb4` of [reilabs/sunspot](https://github.com/reilabs/sunspot) |
| Go | `1.26.2` (required to build Sunspot) |
| Rust | stable (1.x) |

The Dockerfile installs Nargo `1.0.0-beta.18` and Go `1.26.2`, checks out that Sunspot commit, and builds it with `go build -o sunspot .` inside `go/`.

## Prerequisites

### Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Nargo 1.0.0-beta.18

Official installer: [noirup](https://github.com/noir-lang/noirup).

```bash
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
source ~/.bashrc
noirup --version 1.0.0-beta.18
nargo --version
```

### Go 1.26.2

```bash
go version
```

### Sunspot

```bash
git clone https://github.com/reilabs/sunspot.git ~/sunspot
cd ~/sunspot
git checkout a58077e58366e0b2750b032652f2588a6776abb4
cd go
go build -o sunspot .
sudo mv sunspot /usr/local/bin/
sunspot --help
```

## Setup and run

No `SIGNING_KEY`. The server only reads `PORT` and `WORKER_COUNT`.

```bash
cat > .env <<EOF
PORT=4000
WORKER_COUNT=4
EOF

cargo run --release
```

Startup logs from this code:

```
nargo compile done
running sunspot compile...
sunspot compile done
running sunspot setup...
sunspot setup done
workers: 4
server running on port 4000
```

If the circuit JSON already exists and is not older than `main.nr`, the log says `circuit already compiled, skipping compile`. If `proofsystem.ccs` or `proofsystem.vk` already exist, compile or setup is skipped.

### Docker

```bash
docker build -t zk-sunspot-no-signature .
docker run --env-file .env -p 4000:4000 zk-sunspot-no-signature
```

The image binary name is `proofsystem`, which is the Cargo package name.

## API

`POST /proof/prove` and `POST /proof/verify`.

```json
{
  "data": [10, 20, 30, 400, 50],
  "constraints": [
    { "operation": 0, "value": 10,  "attribute_type": 0 },
    { "operation": 1, "value": 25,  "attribute_type": 0 },
    { "operation": 3, "value": 10,  "attribute_type": 0 },
    { "operation": 4, "value": 400, "attribute_type": 0 },
    { "operation": 2, "value": 55,  "attribute_type": 0 }
  ]
}
```

Prove response fields: `success`, `proof`, `vk`, `publicInputs`.

Verify request uses those three proof fields. Response: `success`, `valid`.

## Constraints

From `proofsystem/src/main.nr` in this folder:

| Code | Meaning |
|---|---|
| 0 | equal |
| 1 | less than |
| 2 | less than or equal |
| 3 | greater than |
| 4 | greater than or equal |

`attribute_type` `0` is an integer. `2` is a boolean (the value must be 0 or 1). The handler rejects any other attribute type.

## What the server runs

1. `nargo compile` in `proofsystem/` when the circuit JSON is missing or older than `main.nr`.
2. `sunspot compile`, then `sunspot setup`, unless the `.ccs` or `.vk` file is already present.
3. Each prove: write `Prover.toml`, `nargo execute`, then `sunspot prove`.
4. Each verify: `sunspot verify`.

## Environment

| Variable | Required | Default |
|---|---|---|
| `PORT` | No | `4000` |
| `WORKER_COUNT` | No | CPU count |
