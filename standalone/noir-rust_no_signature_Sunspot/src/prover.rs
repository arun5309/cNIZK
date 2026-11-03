
use std::{fs, path::Path, process::Command, time::Instant};

use tracing::info;
use uuid::Uuid;

use crate::constants::*;

use crate::types::{Constraint, ProveResult};

use crate::utils::{ints_to_toml, DirCleanup};

fn write_prover_toml(
    toml_dir: &Path,
    data: &[i64],
    constraints: &[Constraint],
) -> Result<(), String> {

    let mut t = String::new();

    t.push_str(&format!(
        "data = {}\n\n",
        ints_to_toml(data)
    ));

    for c in constraints {
        t.push_str("[[constraints]]\n");

        t.push_str(&format!(
            "attribute_type = \"{}\"\n",
            c.attribute_type
        ));

        t.push_str(&format!(
            "operation = \"{}\"\n",
            c.operation
        ));

        t.push_str(&format!(
            "value = \"{}\"\n\n",
            c.value
        ));
    }

    fs::write(
        toml_dir.join("Prover.toml"),
        t,
    )
    .map_err(|e| e.to_string())
}

pub fn precompute_vk(
    project_root: &Path
) -> Result<std::path::PathBuf, String> {

    let target_dir = project_root
        .join("proofsystem")
        .join("target");

    let json_path = target_dir.join(CIRCUIT_JSON);

    let ccs_path = target_dir.join(CIRCUIT_CCS);

    let vk_path = target_dir.join(CIRCUIT_VK);

    if !ccs_path.exists() {

        info!("running sunspot compile...");

        let status = Command::new("sunspot")
            .args([
                "compile",
                json_path.to_str().unwrap(),
            ])
            .status()
            .map_err(|e| format!("sunspot not found: {}", e))?;

        if !status.success() {
            return Err("sunspot compile failed".to_string());
        }

        info!("sunspot compile done");

    } else {
        info!("ccs already exists, skipping compile");
    }

    if !vk_path.exists() {

        info!("running sunspot setup...");

        let status = Command::new("sunspot")
            .args([
                "setup",
                ccs_path.to_str().unwrap(),
            ])
            .status()
            .map_err(|e| format!("sunspot not found: {}", e))?;

        if !status.success() {
            return Err("sunspot setup failed".to_string());
        }

        info!("sunspot setup done");

    } else {
        info!("vk already exists, skipping setup");
    }

    Ok(vk_path)
}

pub fn run_prover(
    project_root: &Path,
    vk_path: &Path,
    data: &[i64],
    constraints: &[Constraint],
) -> Result<ProveResult, String> {

    let circuit_dir = project_root.join("proofsystem");

    let target_dir = circuit_dir.join("target");

    let json_path = target_dir.join(CIRCUIT_JSON);

    let ccs_src = target_dir.join(CIRCUIT_CCS);

    let pk_src = target_dir.join(CIRCUIT_PK);

    let uuid = Uuid::new_v4().to_string();

    let tmp_dir = project_root
        .join("tmp")
        .join(&uuid);

    fs::create_dir_all(&tmp_dir)
        .map_err(|e| e.to_string())?;

    let _cleanup = DirCleanup(tmp_dir.clone());

    let ccs_dst = tmp_dir.join(CIRCUIT_CCS);

    let pk_dst = tmp_dir.join(CIRCUIT_PK);

    fs::copy(&ccs_src, &ccs_dst)
        .map_err(|e| format!("copy ccs: {}", e))?;

    fs::copy(&pk_src, &pk_dst)
        .map_err(|e| format!("copy pk: {}", e))?;

    write_prover_toml(
        &tmp_dir,
        data,
        constraints,
    )?;

    let prover_path = tmp_dir.join("Prover");

    let t = Instant::now();

    let status = Command::new("nargo")
        .args([
            "execute",
            &uuid,
            "--prover-name",
            prover_path.to_str().unwrap(),
        ])
        .current_dir(&circuit_dir)
        .status()
        .map_err(|e| format!("nargo not found: {}", e))?;

    info!("witness generation: {:?}", t.elapsed());

    if !status.success() {
        return Err(format!(
            "nargo execute failed (exit {:?})",
            status.code()
        ));
    }

    let witness = target_dir.join(format!("{}.gz", uuid));

    let t = Instant::now();

    let status = Command::new("sunspot")
        .args([
            "prove",
            json_path.to_str().unwrap(),
            witness.to_str().unwrap(),
            ccs_dst.to_str().unwrap(),
            pk_dst.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("sunspot not found: {}", e))?;

    info!(
        "proof generation (sunspot): {:?}",
        t.elapsed()
    );

    if !status.success() {
        return Err(format!(
            "sunspot prove failed (exit {:?})",
            status.code()
        ));
    }

    let _ = fs::remove_file(&witness);

    let proof = fs::read(
        tmp_dir.join(PROOF_FILE)
    )
    .map_err(|e| format!("read proof: {}", e))?;

    let pw = fs::read(
        tmp_dir.join(PUB_WITNESS_FILE)
    )
    .map_err(|e| format!("read public witness: {}", e))?;

    let vk = fs::read(vk_path)
        .map_err(|e| format!("read vk: {}", e))?;

    Ok(ProveResult {
        proof,
        vk,
        public_inputs: pw,
    })
}
