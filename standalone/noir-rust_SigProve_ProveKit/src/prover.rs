use std::{fs, path::Path, process::Command, time::Instant};
use tracing::info;
use uuid::Uuid;

use crate::constants::*;
use crate::types::{Constraint, ProveResult};
use crate::utils::{bytes_to_toml_array, ints_to_toml, DirCleanup};

fn write_prover_toml(
    toml_dir: &Path,
    data: &[i64],
    constraints: &[Constraint],
    ts: u32,
    signature: &[u8],
    pub_key_x: &[u8],
    pub_key_y: &[u8],
) -> Result<(), String> {
    let mut t = String::new();

    t.push_str(&format!("creation_timestamp = \"{}\"\n\n", ts));
    t.push_str(&format!("data = {}\n\n",      ints_to_toml(data)));
    t.push_str(&format!("pub_key_x = {}\n",   bytes_to_toml_array(pub_key_x)));
    t.push_str(&format!("pub_key_y = {}\n",   bytes_to_toml_array(pub_key_y)));
    t.push_str(&format!("signature = {}\n\n", bytes_to_toml_array(signature)));

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
        .join(CIRCUIT_JSON);

    let keys_dir = project_root.join("keys");
    let pkp_path = keys_dir.join(PROVER_PKP);
    let pkv_path = keys_dir.join(VERIFIER_PKV);

    info!("preparing ProveKit keys → keys/ folder...");
    let t = Instant::now();

    let status = Command::new("provekit-cli")
        .args([
            "prepare",
            circuit_json.to_str().unwrap(),
            "--pkp", pkp_path.to_str().unwrap(),
            "--pkv", pkv_path.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("provekit-cli not found: {}", e))?;

    if !status.success() {
        return Err(format!(
            "provekit-cli prepare failed (exit {:?})",
            status.code()
        ));
    }

    info!("ProveKit keys ready in {:?} → keys/prover.pkp + keys/verifier.pkv",
        t.elapsed());

    Ok((pkp_path, pkv_path))
}

pub fn run_prover(
    project_root: &Path,
    pkp_path: &Path,
    pkv_path: &Path,
    data: &[i64],
    constraints: &[Constraint],
    ts: u32,
    signature: &[u8],
    pub_key_x: &[u8],
    pub_key_y: &[u8],
) -> Result<ProveResult, String> {

    let uuid    = Uuid::new_v4().to_string();
    let tmp_dir = project_root.join("tmp").join(&uuid);
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone());

    write_prover_toml(
        &tmp_dir, data, constraints, ts,
        signature, pub_key_x, pub_key_y,
    )?;

    let prover_toml = tmp_dir.join("Prover.toml");
    let proof_out   = tmp_dir.join(PROOF_NP);

    let t = Instant::now();
    let status = Command::new("provekit-cli")
        .args([
            "prove",
            pkp_path.to_str().unwrap(),
            prover_toml.to_str().unwrap(),
            "-o", proof_out.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("provekit-cli not found: {}", e))?;

    info!("proof generation: {:?}", t.elapsed());

    if !status.success() {
        return Err(format!(
            "provekit-cli prove failed (exit {:?})",
            status.code()
        ));
    }

    let proof_bytes = fs::read(&proof_out).map_err(|e| e.to_string())?;
    let vk_bytes    = fs::read(pkv_path).map_err(|e| e.to_string())?;

    let show_output = Command::new("provekit-cli")
        .args([
            "show-inputs",
            pkv_path.to_str().unwrap(),
            proof_out.to_str().unwrap(),
        ])
        .output()
        .unwrap_or_else(|_| std::process::Output {
            status: std::process::ExitStatus::default(),
            stdout: vec![],
            stderr: vec![],
        });

    let pub_input_bytes = show_output.stdout;

    info!("proof.np size (proof + public inputs embedded): {} bytes ({:.2} KB)",
        proof_bytes.len(), proof_bytes.len() as f64 / 1024.0);
    info!("public_inputs size (extracted via show-inputs): {} bytes ({:.2} KB)",
        pub_input_bytes.len(), pub_input_bytes.len() as f64 / 1024.0);
    info!("verifier.pkv size: {} bytes ({:.2} KB)",
        vk_bytes.len(), vk_bytes.len() as f64 / 1024.0);

        
    Ok(ProveResult {
        proof_np:     proof_bytes,
        verifier_pkv: vk_bytes,
    })
}
