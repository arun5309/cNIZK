use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::{fs, path::Path, process::Command, time::Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::constants::{PROOF_FILE, PUB_INPUT_FILE, VK_FILE};
use crate::utils::DirCleanup;

const FIELD_BYTES:   usize = 32;
const MESSAGE_FIELD: usize = 1 + 15;
const PUB_FIELDS:    usize = MESSAGE_FIELD + 1;

pub fn run_verifier(
    project_root: &Path,
    proof_b64: &str,
    vk_b64: &str,
    pub_b64: &str,
    signed_message: &[u8],
) -> Result<bool, String> {

    let tmp_dir = project_root.join("tmp").join(Uuid::new_v4().to_string());
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone());

    let decode = |s: &str, name: &str| B64.decode(s).map_err(|e| format!("decode {}: {}", name, e));

    let proof_path = tmp_dir.join(PROOF_FILE);
    let vk_path = tmp_dir.join(VK_FILE);
    let pub_path = tmp_dir.join(PUB_INPUT_FILE);

    let pub_bytes = decode(pub_b64, "pub")?;

    fs::write(&proof_path, decode(proof_b64, "proof")?).map_err(|e| e.to_string())?;
    fs::write(&vk_path, decode(vk_b64, "vk")?).map_err(|e| e.to_string())?;
    fs::write(&pub_path, &pub_bytes).map_err(|e| e.to_string())?;

    let t = Instant::now();
    let status = Command::new("bb")
        .args([
            "verify",
            "-p",
            proof_path.to_str().unwrap(),
            "-k",
            vk_path.to_str().unwrap(),
            "-i",
            pub_path.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("bb not found: {}", e))?;

    info!("proof verification: {:?}", t.elapsed());

    if !status.success() {
        return Ok(false);
    }

    let proof_message = message_from_public_inputs(&pub_bytes)?;

    let message_matches = proof_message.as_slice() == signed_message;

    if !message_matches {
        warn!("REJECT: proof message does not match the signed message");
    }

    Ok(message_matches)
}

fn message_from_public_inputs(pub_bytes: &[u8]) -> Result<[u8; 32], String> {
    if pub_bytes.len() != PUB_FIELDS * FIELD_BYTES {
        return Err(format!(
            "publicInputs: expected {} bytes, got {}",
            PUB_FIELDS * FIELD_BYTES,
            pub_bytes.len()
        ));
    }

    let start = MESSAGE_FIELD * FIELD_BYTES;
    let mut out = [0u8; 32];
    out.copy_from_slice(&pub_bytes[start..start + FIELD_BYTES]);
    Ok(out)
}
