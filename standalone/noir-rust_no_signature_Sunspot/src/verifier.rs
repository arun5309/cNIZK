
use base64::{engine::general_purpose::STANDARD as B64, Engine};

use std::{fs, path::Path, process::Command, time::Instant};

use tracing::info;
use uuid::Uuid;

use crate::constants::{
    PROOF_FILE,
    CIRCUIT_VK,
    PUB_WITNESS_FILE,
};

use crate::utils::DirCleanup;


pub fn run_verifier(
    project_root: &Path,
    proof_b64: &str,
    vk_b64: &str,
    pub_b64: &str,
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

    let vk_path = tmp_dir.join(CIRCUIT_VK);

    let pw_path = tmp_dir.join(PUB_WITNESS_FILE);

    fs::write(
        &proof_path,
        decode(proof_b64, "proof")?,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        &vk_path,
        decode(vk_b64, "vk")?,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        &pw_path,
        decode(pub_b64, "public witness")?,
    )
    .map_err(|e| e.to_string())?;

    let t = Instant::now();

    let status = Command::new("sunspot")
        .args([
            "verify",
            vk_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
            pw_path.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("sunspot not found: {}", e))?;

    info!(
        "proof verification (sunspot): {:?}",
        t.elapsed()
    );

    Ok(status.success())
}
