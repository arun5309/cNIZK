use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::{fs, path::Path, process::Command, time::Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::constants::{
    PROOF_FILE,
    CIRCUIT_VK,
    PUB_WITNESS_FILE,
};

use crate::utils::{sunspot_bin, DirCleanup};

const HEADER_BYTES:  usize = 12;
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

    let tmp_dir = project_root
        .join("tmp")
        .join(Uuid::new_v4().to_string());

    fs::create_dir_all(&tmp_dir)
        .map_err(|e| e.to_string())?;

    let _cleanup = DirCleanup(tmp_dir.clone());

    let decode = |s: &str, name: &str| {
        B64.decode(s)
            .map_err(|e| format!("decode {}: {}", name, e))
    };

    let proof_path = tmp_dir.join(PROOF_FILE);
    let vk_path    = tmp_dir.join(CIRCUIT_VK);
    let pw_path    = tmp_dir.join(PUB_WITNESS_FILE);

    fs::write(&proof_path, decode(proof_b64, "proof")?)
        .map_err(|e| e.to_string())?;

    fs::write(&vk_path, decode(vk_b64, "vk")?)
        .map_err(|e| e.to_string())?;

    let pw_bytes = decode(pub_b64, "public witness")?;

    fs::write(&pw_path, &pw_bytes)
        .map_err(|e| e.to_string())?;

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

    info!(
        "proof verification (sunspot): {:?}",
        t.elapsed()
    );

    if !status.success() {
        return Ok(false);
    }

    let proof_message = message_from_public_witness(&pw_bytes)?;

    let message_matches = proof_message.as_slice() == signed_message;

    if !message_matches {
        warn!("REJECT: proof message does not match the signed message");
    }

    Ok(message_matches)
}

fn message_from_public_witness(pw: &[u8]) -> Result<[u8; 32], String> {
    if pw.len() != HEADER_BYTES + PUB_FIELDS * FIELD_BYTES {
        return Err(format!(
            "publicWitness: expected {} bytes, got {}",
            HEADER_BYTES + PUB_FIELDS * FIELD_BYTES,
            pw.len()
        ));
    }

    let be32 = |o: usize| u32::from_be_bytes([pw[o], pw[o + 1], pw[o + 2], pw[o + 3]]) as usize;
    if be32(0) != PUB_FIELDS || be32(4) != 0 || be32(8) != PUB_FIELDS {
        return Err(format!(
            "publicWitness: unexpected header (nbPublic={}, nbSecret={}, len={}), expected ({}, 0, {})",
            be32(0), be32(4), be32(8), PUB_FIELDS, PUB_FIELDS
        ));
    }

    let start = HEADER_BYTES + MESSAGE_FIELD * FIELD_BYTES;
    let mut out = [0u8; 32];
    out.copy_from_slice(&pw[start..start + FIELD_BYTES]);
    Ok(out)
}
