use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::{fs, path::Path, process::Command, time::Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::utils::{sunspot_bin, DirCleanup};

const HEADER_BYTES:    usize = 12;
const FIELD_BYTES:     usize = 32;
const PUB_KEY_X_FIELD: usize = 15 + 1;
const PUB_KEY_Y_FIELD: usize = PUB_KEY_X_FIELD + 32;
const PUB_FIELDS:      usize = PUB_KEY_Y_FIELD + 32;

pub fn run_verifier(
    project_root: &Path,
    proof_b64: &str,
    vk_b64: &str,
    pub_witness_b64: &str,
    issuer_pub_key_x: &[u8; 32],
    issuer_pub_key_y: &[u8; 32],
) -> Result<bool, String> {

    let tmp_dir = project_root.join("tmp").join(Uuid::new_v4().to_string());
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone());

    let decode = |s: &str, name: &str|
        B64.decode(s).map_err(|e| format!("decode {}: {}", name, e));

    let vk_path    = tmp_dir.join("circuit.vk");
    let proof_path = tmp_dir.join("circuit.proof");
    let pw_path    = tmp_dir.join("circuit.pw");

    let pw_bytes = decode(pub_witness_b64, "publicWitness")?;

    fs::write(&vk_path,    decode(vk_b64,    "vk")?).map_err(|e| e.to_string())?;
    fs::write(&proof_path, decode(proof_b64, "proof")?).map_err(|e| e.to_string())?;
    fs::write(&pw_path,    &pw_bytes).map_err(|e| e.to_string())?;

    let t = Instant::now();
    let sunspot = sunspot_bin(project_root);
    let status = Command::new(&sunspot)
        .args([
            "verify",
            vk_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
            pw_path.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("sunspot not found at {}: {}", sunspot.display(), e))?;

    info!("proof verification (sunspot groth16): {:?}", t.elapsed());

    if !status.success() {
        return Ok(false);
    }

    let proof_pub_key_x = pub_key_from_fields(&pw_bytes, PUB_KEY_X_FIELD, "pub_key_x")?;
    let proof_pub_key_y = pub_key_from_fields(&pw_bytes, PUB_KEY_Y_FIELD, "pub_key_y")?;

    let issuer_key_matches =
        proof_pub_key_x == *issuer_pub_key_x && proof_pub_key_y == *issuer_pub_key_y;

    if !issuer_key_matches {
        warn!(
            "REJECT: proof is signed under a different public key. \
             expected x=0x{} y=0x{}, proof has x=0x{} y=0x{}",
            hex::encode(issuer_pub_key_x),
            hex::encode(issuer_pub_key_y),
            hex::encode(proof_pub_key_x),
            hex::encode(proof_pub_key_y),
        );
    }

    Ok(issuer_key_matches)
}

fn pub_key_from_fields(pw_bytes: &[u8], first_field: usize, name: &str) -> Result<[u8; 32], String> {
    if pw_bytes.len() != HEADER_BYTES + PUB_FIELDS * FIELD_BYTES {
        return Err(format!(
            "publicWitness: expected {} bytes, got {}",
            HEADER_BYTES + PUB_FIELDS * FIELD_BYTES,
            pw_bytes.len()
        ));
    }

    let be32 = |o: usize| u32::from_be_bytes([pw_bytes[o], pw_bytes[o + 1], pw_bytes[o + 2], pw_bytes[o + 3]]) as usize;
    if be32(0) != PUB_FIELDS || be32(4) != 0 || be32(8) != PUB_FIELDS {
        return Err(format!(
            "publicWitness: unexpected header (nbPublic={}, nbSecret={}, len={}), expected ({}, 0, {})",
            be32(0), be32(4), be32(8), PUB_FIELDS, PUB_FIELDS
        ));
    }

    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let start = HEADER_BYTES + (first_field + i) * FIELD_BYTES;
        let field = &pw_bytes[start..start + FIELD_BYTES];
        if field[..FIELD_BYTES - 1].iter().any(|&b| b != 0) {
            return Err(format!("{}[{}] is not a byte value", name, i));
        }
        *slot = field[FIELD_BYTES - 1];
    }
    Ok(out)
}
