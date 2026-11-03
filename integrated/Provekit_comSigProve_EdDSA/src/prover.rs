use std::{fs, path::Path, process::{Command, Output}, time::Instant};
use tracing::info;
use uuid::Uuid;

use crate::constants::*;
use crate::types::{Constraint, ProveResult};
use crate::utils::{bytes_to_toml_array, field_to_toml, ints_to_toml, DirCleanup};

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

fn write_prover_toml(
    toml_dir: &Path,
    data: &[u64],
    constraints: &[Constraint],
    ts: u32,
    loan_amount: u64,
    randomness: &[u8; 16],
    message: &[u8; 32],
) -> Result<(), String> {
    let mut t = String::new();

    t.push_str(&format!("creation_timestamp = \"{}\"\n\n", ts));
    t.push_str(&format!("data = {}\n\n",       ints_to_toml(data)));
    t.push_str(&format!("randomness = {}\n\n", bytes_to_toml_array(randomness)));
    t.push_str(&format!("loan_amount = \"{}\"\n\n", loan_amount));
    t.push_str(&format!("message = {}\n\n",    field_to_toml(message)));

    for c in constraints {
        t.push_str("[[constraints]]\n");
        t.push_str(&format!("attribute_type = \"{}\"\n", c.attribute_type));
        t.push_str(&format!("operation      = \"{}\"\n", c.operation));
        t.push_str(&format!("value          = \"{}\"\n\n", c.value));
    }

    fs::write(toml_dir.join("Prover.toml"), t).map_err(|e| e.to_string())
}

pub fn precompute_keys(
    project_root: &Path,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    let circuit_json = project_root
        .join("proofsystem")
        .join("target")
        .join("proofsystem.json");

    let keys_dir = project_root.join("keys");
    fs::create_dir_all(&keys_dir).map_err(|e| e.to_string())?;

    let pkp_path = keys_dir.join(PROVER_PKP);
    let pkv_path = keys_dir.join(VERIFIER_PKV);

    info!("preparing ProveKit keys → keys/ folder...");
    let t = Instant::now();

    let output = Command::new("provekit-cli")
        .args([
            "prepare",
            circuit_json.to_str().unwrap(),
            "--pkp", pkp_path.to_str().unwrap(),
            "--pkv", pkv_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("provekit-cli not found: {}", e))?;

    if !output.status.success() {
        return Err(command_output_error("provekit-cli prepare", &output));
    }

    info!(
        "ProveKit keys ready in {:?} → keys/prover.pkp + keys/verifier.pkv",
        t.elapsed()
    );
    Ok((pkp_path, pkv_path))
}

pub fn run_prover(
    project_root: &Path,
    pkp_path: &Path,
    pkv_path: &Path,
    data: &[u64],
    constraints: &[Constraint],
    ts: u32,
    loan_amount: u64,
    randomness: &[u8; 16],
    message: &[u8; 32],
    signature: &[u8],
) -> Result<ProveResult, String> {

    let uuid    = Uuid::new_v4().to_string();
    let tmp_dir = project_root.join("tmp").join(&uuid);
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone());

    write_prover_toml(&tmp_dir, data, constraints, ts, loan_amount, randomness, message)?;

    let prover_toml = tmp_dir.join("Prover.toml");
    let proof_out   = tmp_dir.join(PROOF_NP);

    let t = Instant::now();
    let output = Command::new("provekit-cli")
        .args([
            "prove",
            "-o", proof_out.to_str().unwrap(),
            pkp_path.to_str().unwrap(),
            prover_toml.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("provekit-cli not found: {}", e))?;

    info!("proof generation: {:?}", t.elapsed());

    if !output.status.success() {
        return Err(command_output_error("provekit-cli prove", &output));
    }

    let proof_bytes = fs::read(&proof_out).map_err(|e| e.to_string())?;
    let vk_bytes    = fs::read(pkv_path).map_err(|e| e.to_string())?;

    Ok(ProveResult {
        proof_np:     proof_bytes,
        verifier_pkv: vk_bytes,
        message:       message.to_vec(),
        signature:     signature.to_vec(),
    })
}
