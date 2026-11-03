use std::str::FromStr;

use ark_bn254::Fr;
use ark_ff::{BigInteger, Field, One, PrimeField, Zero};
use light_poseidon::{Poseidon, PoseidonBytesHasher};
use num_bigint::BigUint;
use rand::RngCore;

// Baby Jubjub subgroup order from noir-lang/eddsa.
const SUBORDER: &str =
    "2736030358979909402780800718157159386076813972158567259200215660948447373041";

/* Full generator G, x. The public key is sk * G.
 From noir-lang/ec v0.1.2 `src/consts/te.nr` line 21 (`bjj.curve.gen`):
 https://github.com/noir-lang/ec/blob/v0.1.2/src/consts/te.nr#L21
 which eddsa v0.1.3 uses: https://github.com/noir-lang/eddsa/tree/v0.1.3
 Same point as noir-edwards `BabyJubJub::one()` (`bjj.nr` `gen()`):
 https://github.com/noir-lang/noir-edwards/blob/main/src/bjj.nr
 x = 0x023343e3445b673d38bcba38f25645adb494b1255b1162bb40f41a59f4d4b45e. */

const BASE_X: &str =
    "995203441582195749578291179787384436505546430278305826713579947235728471134";

/* Full generator G, y.
 From noir-lang/ec v0.1.2 `src/consts/te.nr` line 22:
 https://github.com/noir-lang/ec/blob/v0.1.2/src/consts/te.nr#L22
 Same point as noir-edwards `bjj.nr` `gen()` y:
 https://github.com/noir-lang/noir-edwards/blob/main/src/bjj.nr
 y = 0x0c19139cb84c680a6e14116da06056174a0cfa121e6e5c2450f87d64fc000001. */
const BASE_Y: &str =
    "5472060717959818805561601436314318772137091100104008585924551046643952123905";
// B8 = 8 * G. The signature point R8 is a multiple of B8.
const BASE8_X: &str =
    "5299619240641551281634865583518297030282874472190772894086521144482721001553";
const BASE8_Y: &str =
    "16950150798460657717958625567821834550301663161624707787222815936182638968203";

const CURVE_A: u64 = 168700;
const CURVE_D: u64 = 168696;

// R8.x || R8.y || S, each as a 32-byte big-endian integer.
const SIGNATURE_LEN: usize = 96;

#[derive(Clone, Copy, PartialEq)]
struct Point {
    x: Fr,
    y: Fr,
}

impl Point {
    fn generator() -> Self {
        Self {
            x: Fr::from_str(BASE_X).unwrap(),
            y: Fr::from_str(BASE_Y).unwrap(),
        }
    }

    fn base8() -> Self {
        Self {
            x: Fr::from_str(BASE8_X).unwrap(),
            y: Fr::from_str(BASE8_Y).unwrap(),
        }
    }

    fn identity() -> Self {
        Self {
            x: Fr::zero(),
            y: Fr::one(),
        }
    }

    fn is_identity(self) -> bool {
        self.x.is_zero() && self.y.is_one()
    }

    fn is_on_curve(self) -> bool {
        let a = Fr::from(CURVE_A);
        let d = Fr::from(CURVE_D);
        let x2 = self.x.square();
        let y2 = self.y.square();
        a * x2 + y2 == Fr::one() + d * x2 * y2
    }

    fn add(self, other: Self) -> Self {
        let a = Fr::from(CURVE_A);
        let d = Fr::from(CURVE_D);
        let x1x2 = self.x * other.x;
        let y1y2 = self.y * other.y;
        let dt = d * x1x2 * y1y2;
        let x_den = Fr::one() + dt;
        let y_den = Fr::one() - dt;
        Self {
            x: (self.x * other.y + self.y * other.x)
                * x_den.inverse().expect("Baby Jubjub x denominator"),
            y: (y1y2 - a * x1x2)
                * y_den.inverse().expect("Baby Jubjub y denominator"),
        }
    }

    fn double(self) -> Self {
        self.add(self)
    }

    fn mul(self, scalar: &BigUint) -> Self {
        let mut acc = Self::identity();
        let mut base = self;
        for byte in scalar.to_bytes_le() {
            for bit in 0..8 {
                if (byte >> bit) & 1 == 1 {
                    acc = acc.add(base);
                }
                base = base.double();
            }
        }
        acc
    }
}

fn suborder() -> BigUint {
    BigUint::parse_bytes(SUBORDER.as_bytes(), 10).unwrap()
}

fn fr_to_be_bytes(value: Fr) -> [u8; 32] {
    let bytes = value.into_bigint().to_bytes_be();
    let mut out = [0u8; 32];
    let start = 32 - bytes.len();
    out[start..].copy_from_slice(&bytes);
    out
}

fn fr_from_be_bytes(bytes: &[u8]) -> Result<Fr, String> {
    if bytes.len() != 32 {
        return Err(format!("expected 32 bytes, got {}", bytes.len()));
    }
    let value = Fr::from_be_bytes_mod_order(bytes);
    if fr_to_be_bytes(value).as_slice() != bytes {
        return Err("integer is outside the BN254 field".to_string());
    }
    Ok(value)
}

fn bytes_mod_order(bytes: &[u8]) -> BigUint {
    BigUint::from_bytes_be(bytes) % suborder()
}

fn scalar_to_be_bytes(scalar: &BigUint) -> [u8; 32] {
    let bytes = scalar.to_bytes_be();
    let mut out = [0u8; 32];
    let start = 32 - bytes.len();
    out[start..].copy_from_slice(&bytes);
    out
}

fn secret_scalar(sk_bytes: &[u8; 32]) -> Result<BigUint, String> {
    let scalar = bytes_mod_order(sk_bytes);
    if scalar == BigUint::from(0u8) {
        return Err("signing key is zero modulo the Baby Jubjub subgroup order".to_string());
    }
    Ok(scalar)
}

fn challenge(r8: Point, public_key: Point, message: Fr) -> BigUint {
    let parts = [
        fr_to_be_bytes(r8.x),
        fr_to_be_bytes(r8.y),
        fr_to_be_bytes(public_key.x),
        fr_to_be_bytes(public_key.y),
        fr_to_be_bytes(message),
    ];
    let mut poseidon = Poseidon::<Fr>::new_circom(5).expect("Poseidon width 5");
    let hash: [u8; 32] = poseidon
        .hash_bytes_be(&[&parts[0], &parts[1], &parts[2], &parts[3], &parts[4]])
        .expect("Poseidon hash")
        .try_into()
        .expect("Poseidon hash is 32 bytes");
    bytes_mod_order(&hash)
}

fn pack_randomness_to_field(randomness: &[u8; 16]) -> [u8; 32] {
    let mut field = [0u8; 32];
    field[16..32].copy_from_slice(randomness); 
    field
}

pub fn poseidon_hash_payload(
    data: &[i64],
    creation_timestamp: u32,
    randomness: &[u8; 16],
) -> [u8; 32] {
    let make_field = |v: i64| -> [u8; 32] {
        let mut f = [0u8; 32];
        let val = v as u32;
        f[28] = (val >> 24) as u8;
        f[29] = (val >> 16) as u8;
        f[30] = (val >> 8)  as u8;
        f[31] = (val & 0xff) as u8;
        f
    };

    let fields: Vec<[u8; 32]> = vec![
        make_field(data[0]),
        make_field(data[1]),
        make_field(data[2]),
        make_field(data[3]),
        make_field(data[4]),
        {
            let mut f = [0u8; 32];
            let ts = creation_timestamp;
            f[28] = (ts >> 24) as u8;
            f[29] = (ts >> 16) as u8;
            f[30] = (ts >> 8)  as u8;
            f[31] = (ts & 0xff) as u8;
            f
        },
        pack_randomness_to_field(randomness),
    ];

    let mut acc = fields[0];
    for i in 1..fields.len() {
        let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();
        acc = poseidon
            .hash_bytes_be(&[&acc, &fields[i]])
            .unwrap()
            .try_into()
            .unwrap();
    }
    acc
}

pub fn sign_hash(sk_bytes: &[u8; 32], hash: &[u8; 32]) -> Result<Vec<u8>, String> {
    let sk = secret_scalar(sk_bytes)?;
    let message = fr_from_be_bytes(hash)?;
    let public_key = Point::generator().mul(&sk);

    let order = suborder();
    let nonce = loop {
        let mut nonce_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let candidate = BigUint::from_bytes_be(&nonce_bytes);
        if !candidate.is_zero() && candidate < order {
            break candidate;
        }
    };

    let r8 = Point::base8().mul(&nonce);
    let h = challenge(r8, public_key, message);
    let s = (nonce + h * &sk) % order;

    let mut signature = [0u8; SIGNATURE_LEN];
    signature[..32].copy_from_slice(&fr_to_be_bytes(r8.x));
    signature[32..64].copy_from_slice(&fr_to_be_bytes(r8.y));
    signature[64..].copy_from_slice(&scalar_to_be_bytes(&s));
    Ok(signature.to_vec())
}

pub fn get_pub_key_xy(sk_bytes: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let sk = secret_scalar(sk_bytes).expect("invalid Baby Jubjub signing key");
    let public_key = Point::generator().mul(&sk);
    (fr_to_be_bytes(public_key.x), fr_to_be_bytes(public_key.y))
}

pub fn verify_signature(
    message: &[u8],
    signature: &[u8],
    pub_key_x: &[u8],
    pub_key_y: &[u8],
) -> Result<bool, String> {
    if message.len() != 32 {
        return Err(format!("message must be 32 bytes, got {}", message.len()));
    }
    if signature.len() != SIGNATURE_LEN {
        return Err(format!(
            "signature must be {} bytes, got {}",
            SIGNATURE_LEN,
            signature.len()
        ));
    }
    if pub_key_x.len() != 32 || pub_key_y.len() != 32 {
        return Err("pub_key_x and pub_key_y must be 32 bytes each".to_string());
    }

    let s = BigUint::from_bytes_be(&signature[64..]);
    if s >= suborder() {
        return Ok(false);
    }

    let public_key = Point {
        x: fr_from_be_bytes(pub_key_x)?,
        y: fr_from_be_bytes(pub_key_y)?,
    };
    let r8 = Point {
        x: fr_from_be_bytes(&signature[..32])?,
        y: fr_from_be_bytes(&signature[32..64])?,
    };
    if !public_key.is_on_curve() || !r8.is_on_curve() {
        return Ok(false);
    }

    let public_key_mul_8 = public_key.double().double().double();
    if public_key_mul_8.is_identity() {
        return Ok(false);
    }

    let message = fr_from_be_bytes(message)?;
    let h = challenge(r8, public_key, message);
    let left = Point::base8().mul(&s);
    let right = r8.add(public_key_mul_8.mul(&h));
    Ok(left == right)
}
