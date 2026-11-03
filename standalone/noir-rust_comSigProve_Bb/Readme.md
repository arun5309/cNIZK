
# Noir ZK Proof Service (Outside-Circuit) — Barretenberg

A ZK proof service where **ECDSA signatures are verified at the Rust/API level** using secp256k1. The server signs a Poseidon hash of data and timestamp, verifies the signature in Rust before proof generation, then passes only the data and constraints to the Noir circuit — which solely enforces constraint logic in ZK, without any signature verification inside the circuit.

**Key Features:**
- Outside-circuit ECDSA signature verification (secp256k1, handled in Rust)
- Poseidon hash commitment (BN254)
- Constraint system: equality, range, boolean validations
- Built with Rust (Axum) + Noir + Barretenberg
- Concurrent proof generation with worker pool
- REST API with proof generation and verification

---

## Inside vs Outside Signature

| | Inside Signature | Outside Signature (this project) |
|---|---|---|
| ECDSA verification | Inside `main.nr` circuit | Rust API layer |
| Circuit complexity | Higher (more constraints) | Lower (constraints only) |
| Proof generation time | Slower | Faster |
| `main.nr` contains | Constraints + ECDSA | Constraints only |

---

## Tool Versions

| Tool | Version |
|---|---|
| Nargo / Noir | `1.0.0-beta.19` (recommended), `1.0.0-beta.18` |
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

### 2. Nargo 1.0.0-beta.19 or 1.0.0-beta.18

```bash
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
source ~/.bashrc
noirup --version 1.0.0-beta.19
nargo --version
```

Expected output:
```
nargo version = 1.0.0-beta.19
noirc version = 1.0.0-beta.19+...
```

If you want `beta.18`, switch instead to:
```bash
noirup --version 1.0.0-beta.18
```

`1.0.0-beta.19` is recommended because this project runs perfectly on that version, but `1.0.0-beta.18` is also supported.

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
- If you select `noirup --version 1.0.0-beta.19`, then running `bbup` will fetch the matching Barretenberg version for that Noir release.
- If you select `noirup --version 1.0.0-beta.18`, then running `bbup` will also automatically install the correct `bb` version for that Noir release.

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
docker build -t zk-barretenberg-outside .
docker run --env-file .env -p 4000:4000 zk-barretenberg-outside
```

> First container startup is slow (~3–5 min) for circuit compile and key generation. Wait for `server running on port 4000` before sending requests.

---

## How It Works

```
POST /proof/prove { data[5], constraints[5] }
        |
        +--> Rust API (Axum)
              |
              |-- generate 16 random bytes (randomness)
              |-- Poseidon hash(data + timestamp + randomness)   [signer.rs]
              |-- secp256k1 ECDSA sign(hash)                     [signer.rs]
              |
              +--> Prover [prover.rs]
                    |
                    |-- Write Prover.toml
                    |   (data, constraints, timestamp, randomness, message)
                    |   NOTE: signature NOT sent to circuit
                    |
                    |-- nargo execute
                    |   -> witness generation (target/<uuid>.gz)
                    |
                    +-- bb prove
                        Noir circuit verifies in ZK:
                          1. All 5 constraints satisfied
                          2. Poseidon hash recomputed and matched
                        -> signature is NOT verified inside circuit

        Returns: proof + vk + publicInputs + message + signature + pub_key_x + pub_key_y

POST /proof/verify { proof, vk, publicInputs, message, signature, pub_key_x, pub_key_y }
        |
        |-- Step 1: verify secp256k1 ECDSA signature at API level
        |   if invalid -> return immediately { valid: false, signature_valid: false }
        |
        +-- Step 2: bb verify proof
              -> { valid: true, signature_valid: true }
```

**Data is a private witness — never revealed in the proof.**
**Signature is verified in Rust before proof generation — not part of the ZK circuit.**

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
  "publicInputs": "<base64>",
  "message": "<base64>",
  "signature": "<base64>",
  "pub_key_x": "<base64>",
  "pub_key_y": "<base64>"
}
```

> `message`, `signature`, `pub_key_x`, `pub_key_y` must all be passed back to `/proof/verify`.

### `POST /proof/verify`

**Request:**
```json
{
  "proof": "<base64>",
  "vk": "<base64>",
  "publicInputs": "<base64>",
  "message": "<base64>",
  "signature": "<base64>",
  "pub_key_x": "<base64>",
  "pub_key_y": "<base64>"
}
```

**Response:**
```json
{ "success": true, "valid": true, "signature_valid": true }
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
docker build -t zk-barretenberg-outside .
docker stop $(docker ps -q)
docker run --env-file .env -p 4000:4000 zk-barretenberg-outside
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
noirup --version 1.0.0-beta.19
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
EOF



