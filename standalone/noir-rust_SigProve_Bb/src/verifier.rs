use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::{fs, path::Path, process::Command, time::Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::constants::{PROOF_FILE, PUB_INPUT_FILE, VK_FILE};
use crate::utils::DirCleanup;

const FIELD_BYTES:     usize = 32;
const PUB_KEY_X_FIELD: usize = 15 + 1;
const PUB_KEY_Y_FIELD: usize = PUB_KEY_X_FIELD + 32;
const PUB_FIELDS:      usize = PUB_KEY_Y_FIELD + 32;

pub fn run_verifier(
    project_root: &Path,
    proof_b64: &str,
    vk_b64: &str,
    pub_b64: &str,
    issuer_pub_key_x: &[u8; 32],
    issuer_pub_key_y: &[u8; 32],
) -> Result<bool, String> {

    let tmp_dir = project_root.join("tmp").join(Uuid::new_v4().to_string());
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone()); 

    let decode = |s: &str, name: &str|
        B64.decode(s).map_err(|e| format!("decode {}: {}", name, e));

    let proof_path = tmp_dir.join(PROOF_FILE);
    let vk_path    = tmp_dir.join(VK_FILE);
    let pub_path   = tmp_dir.join(PUB_INPUT_FILE);

    let pub_bytes = decode(pub_b64, "pub")?;

    fs::write(&proof_path, decode(proof_b64, "proof")?).map_err(|e| e.to_string())?;
    fs::write(&vk_path,    decode(vk_b64,    "vk")?).map_err(|e| e.to_string())?;
    fs::write(&pub_path,   &pub_bytes).map_err(|e| e.to_string())?;

    let t = Instant::now();

    let status = Command::new("bb")
    .args([
        "verify",
        "--scheme", "ultra_honk",           
        "-p", proof_path.to_str().unwrap(),
        "-k", vk_path.to_str().unwrap(),
        "-i", pub_path.to_str().unwrap(),
    ])
    .status() 
        .map_err(|e| format!("bb not found: {}", e))?;

    info!("proof verification: {:?}", t.elapsed());

    if !status.success() {
        return Ok(false);
    }

    let proof_pub_key_x = pub_key_from_fields(&pub_bytes, PUB_KEY_X_FIELD, "pub_key_x")?;
    let proof_pub_key_y = pub_key_from_fields(&pub_bytes, PUB_KEY_Y_FIELD, "pub_key_y")?;

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

fn pub_key_from_fields(pub_bytes: &[u8], first_field: usize, name: &str) -> Result<[u8; 32], String> {
    if pub_bytes.len() != PUB_FIELDS * FIELD_BYTES {
        return Err(format!(
            "publicInputs: expected {} bytes, got {}",
            PUB_FIELDS * FIELD_BYTES,
            pub_bytes.len()
        ));
    }

    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let start = (first_field + i) * FIELD_BYTES;
        let field = &pub_bytes[start..start + FIELD_BYTES];
        if field[..FIELD_BYTES - 1].iter().any(|&b| b != 0) {
            return Err(format!("{}[{}] is not a byte value", name, i));
        }
        *slot = field[FIELD_BYTES - 1];
    }
    Ok(out)
}
