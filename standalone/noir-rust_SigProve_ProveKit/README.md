# Noir ZK Proof Service (Inside-Circuit) — ProveKit

A ZK proof service where **ECDSA signatures are verified inside the Noir circuit** using secp256r1 (P-256). The server signs a Poseidon hash of data and timestamp, passes inputs to the Noir circuit, which verifies the signature and constraints in-circuit, generating a proof that the data satisfies all conditions without revealing it.

**Key Features:**
- In-circuit ECDSA signature verification (secp256r1 / P-256)
- Poseidon hash commitment (BN254)
- Constraint system: equality, range, boolean validations
- Built with Rust (Axum) + Noir + ProveKit
- Concurrent proof generation with worker pool
- REST API with proof generation and verification

---

## Tool Versions

| Tool | Version |
|---|---|
| Nargo / Noir | `1.0.0-beta.11` |
| provekit-cli | `0.1.4` |
| Rust | stable (1.x) |

---

## Prerequisites

### 1. Rust (stable)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version   # any recent stable version
```

### 2. Nargo 1.0.0-beta.11

```bash
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
source ~/.bashrc
noirup --version 1.0.0-beta.11
nargo --version   # Expected: nargo version = 1.0.0-beta.11
```

### 3. ProveKit CLI 0.1.4

```bash
cargo install --locked provekit-cli --version 0.1.4
provekit-cli --help
```

This `0.1.4` binary does not print a version from `--version`. `provekit-cli --help` confirms the install.

### 4. OpenSSL dev libraries

```bash
# Ubuntu / Debian
sudo apt-get install -y pkg-config libssl-dev

# macOS
brew install openssl pkg-config
```

---

## Setup & Run

### Local

```bash
# Generate a signing key
openssl rand -hex 32

# Create .env — no quotes, no spaces around =
cat > .env <<EOF
PORT=4000
WORKER_COUNT=4
SIGNING_KEY=<paste 64-char hex here>
EOF

cargo run --release
```

**Server is ready when you see:**
```
circuit compiled
ProveKit keys ready in Xs
server running on port 4000
```

> First startup takes 3–5 minutes — compiles the Noir circuit and generates proving keys. Subsequent starts are fast.

### Docker

```bash
# Build image
docker build -t zk-provekit .

# Run
docker run --env-file .env -p 4000:4000 zk-provekit
```

> First container startup is slow (~3–5 min) for circuit compile and key generation. Wait for `server running on port 4000` before sending requests.

---

## How It Works

```
POST /proof/prove { data[5], constraints[5] }
        |
        +--> Rust API (Axum)
              |
              |-- Poseidon hash(data + timestamp)     [signer.rs]
              |-- P-256 ECDSA sign(hash)              [signer.rs]
              |
              +--> Prover [prover.rs]
                    |
                    |-- Write Prover.toml
                    |   (data, constraints, signature, pub_key, timestamp)
                    |
                    +-- provekit-cli prove  prover.pkp  Prover.toml  -o proof.np
                        Noir circuit verifies in ZK:
                          1. All 5 constraints satisfied
                          2. Poseidon hash matches
                          3. ECDSA signature valid
                        -> proof.np (proof + public inputs embedded)

POST /proof/verify { proof, vk }
        |
        +--> provekit-cli verify  verifier.pkv  proof.np  ->  valid: true/false
```

**Data is a private witness — never revealed in the proof.**

---

## API

### `POST /proof/prove`

**Request:**
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

**Response:**
```json
{
  "success": true,
  "proof": "<base64>",
  "vk": "<base64>",
  "publicInputs": ""
}
```

### `POST /proof/verify`

**Request:**
```json
{
  "proof": "<base64>",
  "vk": "<base64>",
  "publicInputs": ""
}
```

**Response:**
```json
{ "success": true, "valid": true }
```

---

## Constraints Reference

| Operation | Code | Meaning |
|---|---|---|
| Equal | 0 | `data[i] == value` |
| Less Than | 1 | `data[i] < value` |
| Less Than or Equal | 2 | `data[i] <= value` |
| Greater Than | 3 | `data[i] > value` |
| Greater Than or Equal | 4 | `data[i] >= value` |

Attribute types: `0` = integer (all ops), `2` = boolean (0 or 1 only). Type `1` is not used.

---

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `SIGNING_KEY` | Yes | — | 64-char hex P-256 private key. Generate: `openssl rand -hex 32` |
| `PORT` | No | `4000` | HTTP listen port |
| `WORKER_COUNT` | No | CPU count | Max concurrent proof generation workers |

---

## Project Structure

```
.
|-- Dockerfile
|-- Cargo.toml
|-- Cargo.lock
|-- .env
|-- src/
|   |-- main.rs          <- server startup, circuit compile, key prep
|   |-- constants.rs     <- array sizes, op codes
|   |-- handlers.rs      <- HTTP route handlers
|   |-- prover.rs        <- ProveKit key gen and proof generation
|   |-- signer.rs        <- Poseidon hash + P-256 ECDSA signing
|   |-- state.rs         <- shared AppState
|   |-- types.rs         <- request/response structs
|   |-- utils.rs         <- Prover.toml writer, helpers
|   +-- verifier.rs      <- proof verification
+-- proofsystem/
    |-- Nargo.toml
    +-- src/
        +-- main.nr      <- ZK circuit: constraints + Poseidon + ECDSA verify
```

---

## Constraint Walkthrough Example

For `data = [10, 20, 30, 400, 50]` with the example constraints:

```
constraints[0]: operation=0 (==),  value=10  ->  10 == 10   PASS
constraints[1]: operation=1 (<),   value=25  ->  20 < 25    PASS
constraints[2]: operation=3 (>),   value=10  ->  30 > 10    PASS
constraints[3]: operation=4 (>=),  value=400 ->  400 >= 400 PASS
constraints[4]: operation=2 (<=),  value=55  ->  50 <= 55   PASS
```

All constraints pass — proof is generated. If any constraint fails, `/proof/prove` returns HTTP 500.

---

## Rebuilding After Code Changes

```bash
# Rebuild Docker image (Rust deps cached — fast ~1-2 min for src/ changes)
docker build -t zk-provekit .

# Stop running container
docker stop $(docker ps -q)

# Start new container
docker run --env-file .env -p 4000:4000 zk-provekit
```

> Changing `proofsystem/src/main.nr` or `Nargo.toml` triggers a full circuit recompile on next startup (~3–5 min).

---

## Troubleshooting

**`SIGNING_KEY not set`**
The `.env` file is missing or incorrectly formatted.
```bash
cat .env
# Must look exactly like:
# PORT=4000
# WORKER_COUNT=4
# SIGNING_KEY=ddc7edc0247d8592596f8734f3d494de0b956237a312efc226ed68d8fe5f15e8
```

**`provekit-cli: command not found`**
```bash
cargo install --locked provekit-cli --version 0.1.4
source $HOME/.cargo/env
```

**`nargo: command not found`**
```bash
source ~/.bashrc
noirup --version 1.0.0-beta.11
```

**`provekit-cli compile failed` on startup**
The circuit failed to compile. Common causes: no internet connection (cannot fetch GitHub dependencies), or wrong Nargo version.
```bash
nargo --version   # must be exactly 1.0.0-beta.11
```

**`/proof/prove` returns 500**
Data does not satisfy the constraints — circuit assertion failed. Check each `data[i]` against `constraints[i]`.

**Port already in use**
```bash
docker stop $(docker ps -q)
sudo lsof -i :4000
```

**First startup takes more than 5 minutes**
Normal on low-resource machines. Circuit compilation and UltraPlonk key generation are CPU and memory intensive. Wait it out — only happens once per fresh container.