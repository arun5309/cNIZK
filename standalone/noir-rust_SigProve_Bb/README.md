# Noir ZK Proof Service (Inside-Circuit) — Barretenberg 

A ZK proof service where **ECDSA signatures are verified inside the Noir circuit** using secp256k1. The server signs a Poseidon hash of data and timestamp, passes inputs to the Noir circuit, which verifies the signature and constraints in-circuit, generating a proof that the data satisfies all conditions without revealing it.

**Key Features:**
- In-circuit ECDSA signature verification (secp256k1)
- Poseidon hash commitment (BN254)
- Constraint system: equality, range, boolean validations
- Built with Rust (Axum) + Noir + Barretenberg
- Concurrent proof generation with worker pool
- REST API with proof generation and verification

---

## Tool Versions

| Tool | Version |
|---|---|
| Nargo / Noir | `1.0.0-beta.18` (recommended), `1.0.0-beta.19` |
| Barretenberg (`bb`) | `3.0.0-nightly.20260102` (recommended) |
|  | `4.0.0-nightly.20260120` (also compatible) |
| Rust | stable (1.x) |

---

## Prerequisites

### 1. Rust (stable)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version
```

### 2. Nargo 1.0.0-beta.18 or 1.0.0-beta.19

```bash
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
source ~/.bashrc
noirup --version 1.0.0-beta.18
nargo --version
```

Expected output:
```
nargo version = 1.0.0-beta.18
noirc version = 1.0.0-beta.18+...
```

If you want `beta.19`, switch instead to:
```bash
noirup --version 1.0.0-beta.19
```

`1.0.0-beta.18` is recommended because this project runs perfectly on that version, but `1.0.0-beta.19` is also supported.

### 3. Barretenberg (`bb`) via `bbup`

```bash
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/master/barretenberg/cpp/installation/install | bash
source ~/.bashrc
bbup
bb --version
```

Expected output:
```bash
3.0.0-nightly.20260102
```

Note:
- If you select `noirup --version 1.0.0-beta.18`, then running `bbup` will fetch the matching Barretenberg version for that Noir release.
- If you select `noirup --version 1.0.0-beta.19`, then running `bbup` will also automatically install the correct `bb` version for that Noir release.

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
openssl rand -hex 32

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
bb keys ready
server running on port 4000
```

> First startup takes 3–5 minutes — compiles the Noir circuit and generates proving keys. Subsequent starts are fast.

### Docker

```bash
docker build -t zk-barretenberg .
docker run --env-file .env -p 4000:4000 zk-barretenberg
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
              |-- secp256k1 ECDSA sign(hash)          [signer.rs]
              |
              +--> Prover [prover.rs]
                    |
                    |-- Write Prover.toml
                    |   (data, constraints, signature, pub_key, timestamp)
                    |
                    |-- nargo execute
                    |   -> witness generation (tmp/<uuid>.gz)
                    |
                    +-- bb prove + bb verify
                        Noir circuit verifies in ZK:
                          1. All 5 constraints satisfied
                          2. Poseidon hash matches
                          3. secp256k1 ECDSA signature valid
                        -> proof + public_inputs (tmp/<uuid>/)

POST /proof/verify { proof, vk, public_inputs }
        |
        +--> bb verify  ->  valid: true/false
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
  "public_inputs": "<base64>"
}
```

### `POST /proof/verify`

**Request:**
```json
{
  "proof": "<base64>",
  "vk": "<base64>",
  "public_inputs": "<base64>"
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
| `SIGNING_KEY` | Yes | — | 64-char hex secp256k1 private key. Generate: `openssl rand -hex 32` |
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
|   |-- main.rs
|   |-- constants.rs
|   |-- handlers.rs
|   |-- prover.rs
|   |-- signer.rs
|   |-- state.rs
|   |-- types.rs
|   |-- utils.rs
|   +-- verifier.rs
+-- proofsystem/
    |-- Nargo.toml
    +-- src/
        +-- main.nr
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
docker build -t zk-barretenberg .
docker stop $(docker ps -q)
docker run --env-file .env -p 4000:4000 zk-barretenberg
```

> Changing `proofsystem/src/main.nr` or `Nargo.toml` triggers a full circuit recompile on next startup (~3–5 min).

---

## Troubleshooting

**`SIGNING_KEY not set`**
```bash
cat .env
# Must look exactly like:
# PORT=4000
# WORKER_COUNT=4
# SIGNING_KEY=f3c9e5a2b1d8c4f6a9e0b3c5d7f1a2e4c6b8d9f0a1c3e5b7d9f2a4c6e8b1dc12
```

**`bb: command not found`**
```bash
bbup --version 3.0.0-nightly.20260102
source ~/.bashrc
bb --version
```

**`nargo: command not found`**
```bash
source ~/.bashrc
noirup --version 1.0.0-beta.18
```

**`nargo execute failed` on startup**
No internet connection or wrong Nargo version.
```bash
nargo --version
```

**`/proof/prove` returns 500**
Data does not satisfy the constraints. Check each `data[i]` against `constraints[i]`.

**Port already in use**
```bash
docker stop $(docker ps -q)
sudo lsof -i :4000
```

**First startup takes more than 5 minutes**
Normal on low-resource machines. Circuit compilation and key generation are CPU and memory intensive. Only happens once per fresh container.