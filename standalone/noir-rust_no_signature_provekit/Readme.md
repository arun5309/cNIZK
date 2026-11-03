# Noir ZK Proof Service (No Signature) — ProveKit

This service proves that 5 data values satisfy 5 constraints. There is no signature. The circuit does not hash the data and does not check a public key.

Built with Rust (Axum) + Noir + ProveKit (`provekit-cli`).

## Tool versions

| Tool | Version |
|---|---|
| Nargo / Noir | `1.0.0-beta.11` |
| provekit-cli | `0.1.4` |
| Rust | stable (1.x) |

The Dockerfile installs these same versions. `0.1.4` is a published crates.io release. Newer `provekit-cli` releases use a different Nargo version and a different command line, so this project stays on `0.1.4`.

## Prerequisites

### Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Nargo 1.0.0-beta.11

Official installer: [noirup](https://github.com/noir-lang/noirup).

```bash
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
source ~/.bashrc
noirup --version 1.0.0-beta.11
nargo --version
```

### provekit-cli 0.1.4

Published crate: [provekit-cli 0.1.4](https://crates.io/crates/provekit-cli/0.1.4). Repository: [worldfnd/provekit](https://github.com/worldfnd/provekit).

```bash
cargo install --locked provekit-cli --version 0.1.4
```

This `0.1.4` binary does not print a version from `--version`. Check the install with `provekit-cli --help`.

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
preparing ProveKit keys → keys/ folder...
ProveKit keys ready in ... → keys/prover.pkp + keys/verifier.pkv
workers: 4
server running on port 4000
```

If `proofsystem/target/proofsystem.json` is already newer than `main.nr`, the log says `circuit already compiled, skipping compile`.

### Docker

```bash
docker build -t zk-provekit-no-signature .
docker run --env-file .env -p 4000:4000 zk-provekit-no-signature
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

Prove response fields: `success`, `proof`, `pkv`. There is no `vk` and no `publicInputs`.

Verify request:

```json
{
  "proof": "<base64>",
  "pkv": "<base64>"
}
```

Response: `success`, `valid`.

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

1. `provekit-cli compile` in `proofsystem/` when the circuit JSON is missing or older than `main.nr`. This project does not call `nargo compile`.
2. `provekit-cli prepare` on every start, writing `keys/prover.pkp` and `keys/verifier.pkv`.
3. Each prove: write `Prover.toml`, then `provekit-cli prove`.
4. Each verify: `provekit-cli verify`.

## Environment

| Variable | Required | Default |
|---|---|---|
| `PORT` | No | `4000` |
| `WORKER_COUNT` | No | CPU count |
