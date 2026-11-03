use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::{fs, path::Path, process::Command, time::Instant};
use tracing::info;
use uuid::Uuid;

use crate::constants::{PKV_FILE, PROOF_FILE};
use crate::utils::DirCleanup;

pub fn run_verifier(
    project_root: &Path,
    proof_b64:    &str,
    pkv_b64:      &str,
) -> Result<bool, String> {

    let uuid    = Uuid::new_v4().to_string();
    let tmp_dir = project_root.join("tmp").join(&uuid);
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone());

    let decode = |s: &str, name: &str|
        B64.decode(s).map_err(|e| format!("base64 decode {}: {}", name, e));

    let proof_path = tmp_dir.join(PROOF_FILE);
    let pkv_path   = tmp_dir.join(PKV_FILE);

    fs::write(&proof_path, decode(proof_b64, "proof")?).map_err(|e| e.to_string())?;
    fs::write(&pkv_path,   decode(pkv_b64,   "pkv")?).map_err(|e| e.to_string())?;

    info!("[{}] provekit-cli verify starting...", &uuid[..8]);
    let t = Instant::now();

    let status = Command::new("provekit-cli")
        .args([
            "verify",
            pkv_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("provekit-cli not found: {}", e))?;

    info!("[{}] verification: {:?}", &uuid[..8], t.elapsed());

    Ok(status.success())
}
