# Noir ZK Proof Service (No Signature) — Barretenberg

This service proves that 5 data values satisfy 5 constraints. There is no signature. The circuit does not hash the data and does not check a public key.

Built with Rust (Axum) + Noir + Barretenberg (`bb`).

## Tool versions

| Tool | Version |
|---|---|
| Nargo / Noir | `1.0.0-beta.19` |
| Barretenberg (`bb`) | `3.0.0-nightly.20260102` |
| Rust | stable (1.x) |

The Dockerfile installs these same versions.

## Prerequisites

### Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Nargo 1.0.0-beta.19

Official installer: [noirup](https://github.com/noir-lang/noirup).

```bash
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
source ~/.bashrc
noirup --version 1.0.0-beta.19
nargo --version
```

### Barretenberg (`bb`)

Official installer: [bbup](https://github.com/AztecProtocol/aztec-packages/blob/master/barretenberg/bbup/README.md).

```bash
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/refs/heads/next/barretenberg/bbup/install | bash
source ~/.bashrc
bbup -v 3.0.0-nightly.20260102
bb --version
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
circuit compiled
precomputing verification key...
VK precomputed in ...
workers: 4
server running on port 4000
```

If `proofsystem/target/proofsystem.json` is already newer than `main.nr`, the log says `circuit already compiled, skipping compile` instead of `circuit compiled`.

### Docker

```bash
docker build -t zk-barretenberg-no-signature .
docker run --env-file .env -p 4000:4000 zk-barretenberg-no-signature
```

The image binary name is `proofsystem`, which is the Cargo package name.

## API

`POST /proof/prove` and `POST /proof/verify`.

Request body is 5 integers and 5 constraints. There is no `asset` wrapper and no `loanAmount`.

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

From `proofsystem/src/main.nr`:

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
2. `bb write_vk` once, into `vk/`.
3. Each prove: write `Prover.toml`, `nargo execute`, then `bb prove`.
4. Each verify: `bb verify`.

## Environment

| Variable | Required | Default |
|---|---|---|
| `PORT` | No | `4000` |
| `WORKER_COUNT` | No | CPU count |
