use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::{fs, path::Path, process::Command, time::Instant};
use tracing::info;
use uuid::Uuid;

use crate::constants::{PROOF_FILE, PUB_INPUT_FILE, VK_FILE};
use crate::utils::DirCleanup;

pub fn run_verifier(
    project_root: &Path,
    proof_b64: &str,
    vk_b64: &str,
    pub_b64: &str,
) -> Result<bool, String> {

    let tmp_dir = project_root.join("tmp").join(Uuid::new_v4().to_string());
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone());

    let decode = |s: &str, name: &str|
        B64.decode(s).map_err(|e| format!("decode {}: {}", name, e));

    let proof_path = tmp_dir.join(PROOF_FILE);
    let vk_path    = tmp_dir.join(VK_FILE);
    let pub_path   = tmp_dir.join(PUB_INPUT_FILE);

    fs::write(&proof_path, decode(proof_b64, "proof")?).map_err(|e| e.to_string())?;
    fs::write(&vk_path,    decode(vk_b64,    "vk")?).map_err(|e| e.to_string())?;
    fs::write(&pub_path,   decode(pub_b64,   "pub")?).map_err(|e| e.to_string())?;

    let t = Instant::now();
    let status = Command::new("bb")
        .args([
            "verify",
            "-p", proof_path.to_str().unwrap(),
            "-k", vk_path.to_str().unwrap(),
            "-i", pub_path.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("bb not found: {}", e))?;

    info!("proof verification: {:?}", t.elapsed());

    Ok(status.success())
}
