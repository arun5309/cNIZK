use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::{fs, path::Path, process::{Command, Output}, time::Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::constants::{PROOF_NP, VERIFIER_PKV};
use crate::utils::DirCleanup;

fn command_output_error(command_name: &str, output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    let mut msg = format!("{} failed (exit {:?})", command_name, output.status.code());
    if !stderr.is_empty() {
        msg.push_str(": ");
        msg.push_str(&stderr);
    }
    if !stdout.is_empty() {
        msg.push_str(": ");
        msg.push_str(&stdout);
    }
    msg
}

pub fn run_verifier(
    project_root: &Path,
    proof_b64: &str,
    pkv_b64: &str,
    issuer_pub_key_x: &[u8; 32],
    issuer_pub_key_y: &[u8; 32],
) -> Result<bool, String> {

    let tmp_dir = project_root.join("tmp").join(Uuid::new_v4().to_string());
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone());

    let decode = |s: &str, name: &str|
        B64.decode(s).map_err(|e| format!("decode {}: {}", name, e));

    let proof_path = tmp_dir.join(PROOF_NP);
    let pkv_path   = tmp_dir.join(VERIFIER_PKV);

    fs::write(&proof_path, decode(proof_b64, "proof")?).map_err(|e| e.to_string())?;
    fs::write(&pkv_path,   decode(pkv_b64,   "vk")?).map_err(|e| e.to_string())?;

    let t = Instant::now();
    let status = Command::new("provekit-cli")
        .args([
            "verify",
            pkv_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("provekit-cli not found: {}", e))?;

    info!("proof verification: {:?}", t.elapsed());

    if !status.success() {
        return Ok(false);
    }

    let public_inputs = show_inputs(&pkv_path, &proof_path)?;

    let proof_pub_key_x = parse_named_bytes32(&public_inputs, "pub_key_x")?;
    let proof_pub_key_y = parse_named_bytes32(&public_inputs, "pub_key_y")?;

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

fn show_inputs(pkv_path: &Path, proof_path: &Path) -> Result<String, String> {
    let output = Command::new("provekit-cli")
        .args([
            "show-inputs",
            pkv_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("provekit-cli not found: {}", e))?;

    if !output.status.success() {
        return Err(command_output_error("provekit-cli show-inputs", &output));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_named_bytes32(stdout: &str, name: &str) -> Result<[u8; 32], String> {
    let header = format!("{} ([u8; 32]):", name);
    let mut lines = stdout.lines().map(str::trim);

    if !lines.any(|line| line == header) {
        return Err(format!("{} not found in show-inputs output", name));
    }

    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let line = lines
            .next()
            .ok_or_else(|| format!("{}: expected 32 bytes, output ended at index {}", name, i))?;

        let prefix = format!("[{}] (u8): ", i);
        let value = line
            .strip_prefix(prefix.as_str())
            .ok_or_else(|| format!("{}: expected line starting {:?}, got {:?}", name, prefix, line))?;

        *slot = value
            .trim()
            .parse::<u8>()
            .map_err(|e| format!("{}: byte {} is not a valid u8: {}", name, i, e))?;
    }
    Ok(out)
}
