use std::{fs, path::Path, process::Command, time::Instant};
use tracing::info;
use uuid::Uuid;

use crate::constants::*;
use crate::types::{Constraint, ProveResult};
use crate::utils::{bytes_to_toml, ints_to_toml, sunspot_bin, DirCleanup};

fn input_newer(input: &Path, output: &Path) -> bool {
    match (fs::metadata(output), fs::metadata(input)) {
        (Ok(out_m), Ok(in_m)) => match (in_m.modified(), out_m.modified()) {
            (Ok(in_t), Ok(out_t)) => in_t > out_t,
            _ => false,
        },
        (Err(_), _) => true,
        _ => false,
    }
}

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
    t.push_str(&format!("pub_key_x = {}\n",   bytes_to_toml(pub_key_x)));
    t.push_str(&format!("pub_key_y = {}\n",   bytes_to_toml(pub_key_y)));
    t.push_str(&format!("signature = {}\n\n", bytes_to_toml(signature)));
    for c in constraints {
        t.push_str("[[constraints]]\n");
        t.push_str(&format!("attribute_type = \"{}\"\n", c.attribute_type));
        t.push_str(&format!("operation = \"{}\"\n",      c.operation));
        t.push_str(&format!("value = \"{}\"\n\n",        c.value));
    }
    fs::write(toml_dir.join("Prover.toml"), t).map_err(|e| e.to_string())
}

pub fn precompute_vk(project_root: &Path) -> Result<std::path::PathBuf, String> {
    let target_dir = project_root.join("proofsystem").join("target");
    let json_path  = target_dir.join(CIRCUIT_JSON);
    let ccs_path   = target_dir.join(CIRCUIT_CCS);
    let vk_path    = target_dir.join(CIRCUIT_VK);
    let sunspot = sunspot_bin(project_root);

    if !json_path.exists() {
        return Err("proofsystem/target/proofsystem.json not found — run nargo compile first".to_string());
    }

    if input_newer(&json_path, &ccs_path) {
        info!("running sunspot compile...");
        let status = Command::new(&sunspot)
            .args(["compile", json_path.to_str().unwrap()])
            .status()
            .map_err(|e| format!("sunspot not found at {}: {}", sunspot.display(), e))?;
        if !status.success() {
            return Err("sunspot compile failed".to_string());
        }
        info!("sunspot compile done → {:?}", ccs_path);
    } else {
        info!("ccs already matches json, skipping sunspot compile");
    }

    if input_newer(&ccs_path, &vk_path) {
        info!("running sunspot setup...");
        let status = Command::new(&sunspot)
            .args(["setup", ccs_path.to_str().unwrap()])
            .status()
            .map_err(|e| format!("sunspot not found at {}: {}", sunspot.display(), e))?;
        if !status.success() {
            return Err("sunspot setup failed".to_string());
        }
        info!("sunspot setup done → pk + vk at {:?}", target_dir);
    } else {
        info!("vk already matches ccs, skipping sunspot setup");
    }

    Ok(vk_path)
}

pub fn run_prover(
    project_root: &Path,
    vk_path: &Path,          
    data: &[i64],
    constraints: &[Constraint],
    ts: u32,
    signature: &[u8],
    pub_key_x: &[u8],
    pub_key_y: &[u8],
) -> Result<ProveResult, String> {

    let circuit_dir = project_root.join("proofsystem");
    let target_dir  = circuit_dir.join("target");

    let json_path   = target_dir.join(CIRCUIT_JSON);
    let ccs_src     = target_dir.join(CIRCUIT_CCS);
    let pk_src      = target_dir.join(CIRCUIT_PK);
    let sunspot = sunspot_bin(project_root);

    let uuid    = Uuid::new_v4().to_string();
    let tmp_dir = project_root.join("tmp").join(&uuid);
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone());


    let ccs_dst = tmp_dir.join(CIRCUIT_CCS);   
    let pk_dst  = tmp_dir.join(CIRCUIT_PK);    
    fs::copy(&ccs_src, &ccs_dst)
        .map_err(|e| format!("copy .ccs: {}", e))?;
    fs::copy(&pk_src, &pk_dst)
        .map_err(|e| format!("copy .pk: {}", e))?;


    let proof_out = tmp_dir.join(PROOF_FILE);      
    let pw_out    = tmp_dir.join(PUB_WITNESS_FILE); 

    write_prover_toml(
        &tmp_dir, data, constraints, ts,
        signature, pub_key_x, pub_key_y,
    )?;

    let prover_path = tmp_dir.join("Prover");
    let t = Instant::now();
    let status = Command::new("nargo")
        .args([
            "execute",
            &uuid,
            "--prover-name", prover_path.to_str().unwrap(),
        ])
        .current_dir(&circuit_dir)
        .status()
        .map_err(|e| format!("nargo not found: {}", e))?;

    info!("witness generation: {:?}", t.elapsed());

    if !status.success() {
        return Err(format!("nargo execute failed (exit {:?})", status.code()));
    }

    let witness_path = target_dir.join(format!("{}.gz", uuid));

    let t = Instant::now();
    let status = Command::new(&sunspot)
        .args([
            "prove",
            json_path.to_str().unwrap(),   
            witness_path.to_str().unwrap(), 
            ccs_dst.to_str().unwrap(),      
            pk_dst.to_str().unwrap(),      
        ])
        .status()
        .map_err(|e| format!("sunspot not found at {}: {}", sunspot.display(), e))?;

    info!("proof generation (sunspot groth16): {:?}", t.elapsed());

    let _ = fs::remove_file(&witness_path);

    if !status.success() {
        return Err(format!("sunspot prove failed (exit {:?})", status.code()));
    }

    let proof_bytes = fs::read(&proof_out).map_err(|e| format!("read proof: {}", e))?;
    let vk_bytes    = fs::read(vk_path).map_err(|e| format!("read vk: {}", e))?;
    let pw_bytes    = fs::read(&pw_out).map_err(|e| format!("read public witness: {}", e))?;

    Ok(ProveResult {
        proof:         proof_bytes,
        vk:            vk_bytes,
        public_inputs: pw_bytes,
    })
}
