# Noir ZK Proof Service — Sunspot Backend (Outside Signature)

A ZK proof service where constraints are verified **inside the Noir circuit**, but **ECDSA signature verification happens outside at the API level**. The server signs a Poseidon hash of data, timestamp, and randomness, then returns the proof along with the signature and public key so the client can independently verify the signature.

**Key Features:**
- Constraint verification inside ZK circuit (secp256k1)
- Poseidon hash commitment (BN254) with randomness for uniqueness
- ECDSA signature verified **outside** the circuit at API level
- Built with Rust (Axum) + Noir + Sunspot
- Concurrent proof generation with worker pool
- REST API with proof generation and verification

---

## Tool Versions

| Tool | Version |
|---|---|
| Nargo / Noir | `1.0.0-beta.18` |
| Sunspot | commit `a58077e58366e0b2750b032652f2588a6776abb4` |
| Go | `1.26.2` (required to build Sunspot) |
| Rust | stable (1.x) |

---

## Prerequisites

### 1. Rust (stable)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version
```

### 2. Nargo 1.0.0-beta.18

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

To switch versions if needed:
```bash
noirup --version 1.0.0-beta.18
```

### 3. Go 1.26.2 (required to build Sunspot)

```bash
go version

# If not installed or not 1.26.2:
wget https://go.dev/dl/go1.26.2.linux-amd64.tar.gz
sudo rm -rf /usr/local/go
sudo tar -C /usr/local -xzf go1.26.2.linux-amd64.tar.gz
echo 'export PATH="$PATH:/usr/local/go/bin"' >> ~/.bashrc
source ~/.bashrc
go version
```

### 4. Sunspot (build from source)

```bash
git clone https://github.com/reilabs/sunspot.git ~/sunspot
cd ~/sunspot
git checkout a58077e58366e0b2750b032652f2588a6776abb4
cd go
go build -o sunspot .

# Install to PATH (pick one):
sudo mv sunspot /usr/local/bin/
# OR
mkdir -p ~/bin && mv sunspot ~/bin/
echo 'export PATH="$HOME/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

sunspot --help
```

### 5. OpenSSL dev libraries

```bash
# Ubuntu / Debian
sudo apt-get install -y pkg-config libssl-dev
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
sunspot keys ready
server running on port 4000
```

> First startup takes 3–5 minutes — compiles the Noir circuit and generates proving keys. Subsequent starts are fast.

### Docker

```bash
docker build -t zk-sunspot-outside .
docker run --env-file .env -p 4000:4000 zk-sunspot-outside
```

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
                    |-- nargo execute -> witness (.gz)
                    |
                    +-- sunspot prove -> proof + public_inputs (.pw)
                        Noir circuit verifies in ZK:
                          1. All 5 constraints satisfied
                          2. Poseidon hash recomputed and matched
                        -> signature is NOT verified inside circuit

        Returns: proof + vk + public_inputs + message + signature + pub_key_x + pub_key_y

POST /proof/verify { proof, vk, public_inputs, message, signature, pub_key_x, pub_key_y }
        |
        |-- Step 1: verify ECDSA signature at API level
        |   if invalid -> return immediately { valid: false, signature_valid: false }
        |
        +-- Step 2: sunspot verify proof
              -> { valid: true, signature_valid: true }
```

**Data is a private witness — never revealed in the proof. Signature is verified outside ZK.**

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
{
  "success": true,
  "valid": true,
  "signature_valid": true
}
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
docker build -t zk-sunspot-outside .
docker stop $(docker ps -q)
docker run --env-file .env -p 4000:4000 zk-sunspot-outside
```

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

**`sunspot: command not found`**
```bash
cd ~/sunspot
git checkout a58077e58366e0b2750b032652f2588a6776abb4
cd go
go build -o sunspot .
sudo mv sunspot /usr/local/bin/
```

**`nargo: command not found`**
```bash
source ~/.bashrc
noirup --version 1.0.0-beta.18
```

**`/proof/verify` returns `signature_valid: false`**
The `message`, `signature`, `pub_key_x`, or `pub_key_y` fields were not passed correctly from the `/proof/prove` response.

**`/proof/prove` returns 500**
Data does not satisfy the constraints. Check each `data[i]` against `constraints[i]`.

**Port already in use**
```bash
docker stop $(docker ps -q)
sudo lsof -i :4000
```