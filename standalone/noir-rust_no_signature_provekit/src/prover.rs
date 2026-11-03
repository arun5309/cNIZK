use std::{fs, path::Path, process::Command, time::Instant};
use tracing::info;
use uuid::Uuid;

use crate::constants::*;
use crate::types::{Constraint, ProveResult};
use crate::utils::{ints_to_toml, DirCleanup};

fn write_prover_toml(path: &Path, data: &[i64], constraints: &[Constraint]) -> Result<(), String> {
    let mut t = String::new();
    t.push_str(&format!("data = {}\n\n", ints_to_toml(data)));
    for c in constraints {
        t.push_str("[[constraints]]\n");
        t.push_str(&format!("attribute_type = \"{}\"\n", c.attribute_type));
        t.push_str(&format!("operation = \"{}\"\n",      c.operation));
        t.push_str(&format!("value = \"{}\"\n\n",        c.value));
    }
    fs::write(path, t).map_err(|e| e.to_string())
}

pub fn precompute_keys(
    project_root: &Path,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    let circuit_json = project_root
        .join("proofsystem")
        .join("target")
        .join(CIRCUIT_JSON);

    let keys_dir = project_root.join("keys");
    fs::create_dir_all(&keys_dir).map_err(|e| e.to_string())?;

    let pkp_path = keys_dir.join(PKP_FILE);
    let pkv_path = keys_dir.join(PKV_FILE);

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

    info!(
        "ProveKit keys ready in {:?} → keys/prover.pkp + keys/verifier.pkv",
        t.elapsed()
    );
    Ok((pkp_path, pkv_path))
}

pub fn run_prover(
    project_root: &Path,
    pkp_path:     &Path,
    pkv_path:     &Path,
    data:         &[i64],
    constraints:  &[Constraint],
) -> Result<ProveResult, String> {

    let uuid    = Uuid::new_v4().to_string();
    let tmp_dir = project_root.join("tmp").join(&uuid);
    let out_dir = tmp_dir.join("out");
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone());

    let prover_toml = tmp_dir.join("Prover.toml");
    write_prover_toml(&prover_toml, data, constraints)?;

    let proof_out = out_dir.join(PROOF_FILE);

    info!("[{}] provekit-cli prove starting...", &uuid[..8]);
    let t = Instant::now();

    let status = Command::new("provekit-cli")
        .args([
            "prove",
            "-o", proof_out.to_str().unwrap(),
            pkp_path.to_str().unwrap(),
            prover_toml.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("provekit-cli not found: {}", e))?;

    info!("[{}] proof generation: {:?}", &uuid[..8], t.elapsed());

    if !status.success() {
        return Err(format!("provekit-cli prove failed (exit {:?})", status.code()));
    }

    Ok(ProveResult {
        proof: fs::read(&proof_out).map_err(|e| e.to_string())?,
        pkv:   fs::read(pkv_path).map_err(|e| e.to_string())?,
    })
}
