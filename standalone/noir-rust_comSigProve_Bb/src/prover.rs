use std::{fs, path::Path, process::Command, time::Instant};
use tracing::info;
use uuid::Uuid;
use crate::constants::*;
use crate::types::{Constraint, ProveResult};
use crate::utils::{bytes_to_toml_array, field_to_toml, ints_to_toml, DirCleanup};

fn write_prover_toml(
    toml_dir: &Path,
    data: &[u64],
    constraints: &[Constraint],
    ts: u32,
    randomness: &[u8; 16], 
    message: &[u8; 32],    
) -> Result<(), String> {
    let mut t = String::new();
    t.push_str(&format!("creation_timestamp = \"{}\"\n\n", ts));
    t.push_str(&format!("data = {}\n\n", ints_to_toml(data)));
    t.push_str(&format!("randomness = {}\n\n", bytes_to_toml_array(randomness)));
    t.push_str(&format!("message = {}\n\n", field_to_toml(message)));

    for c in constraints {  
        t.push_str("[[constraints]]\n");
        t.push_str(&format!("attribute_type = \"{}\"\n", c.attribute_type));
        t.push_str(&format!("operation = \"{}\"\n", c.operation));
        t.push_str(&format!("value = \"{}\"\n\n", c.value));
    }

    fs::write(toml_dir.join("Prover.toml"), t).map_err(|e| e.to_string())
}
pub fn precompute_vk(project_root: &Path) -> Result<std::path::PathBuf, String> {
    let circuit_dir = project_root.join("proofsystem");
    let vk_dir = project_root.join("vk");

    info!("precomputing verification key...");

    let t = Instant::now();
    let status = Command::new("bb")
        .args([
            "write_vk",
            "-b",
            circuit_dir
                .join("target")
                .join(CIRCUIT_JSON)
                .to_str()
                .unwrap(),
            "-o",
            vk_dir.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("bb not found: {}", e))?;

    if !status.success() {
        return Err(format!("bb write_vk failed (exit {:?})", status.code()));
    }
    info!(
        "VK precomputed in {:?} — reused for all requests",
        t.elapsed()
    );
    Ok(vk_dir.join(VK_FILE))
}
pub fn run_prover(
    project_root: &Path,
    vk_path: &Path,
    data: &[u64],
    constraints: &[Constraint],
    ts: u32,
    randomness: &[u8; 16],
    message: &[u8; 32],
    signature: &[u8], 
) -> Result<ProveResult, String> {
    let circuit_dir = project_root.join("proofsystem"); 

    let uuid = Uuid::new_v4().to_string();
    let tmp_dir = project_root.join("tmp").join(&uuid);
    let out_dir = tmp_dir.join("out");

    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let _cleanup = DirCleanup(tmp_dir.clone()); 

    write_prover_toml(&tmp_dir, data, constraints, ts, randomness, message)?;
    
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
        return Err(format!("nargo execute failed (exit {:?})", status.code()));
    }

    let witness = circuit_dir.join("target").join(format!("{}.gz", uuid));

    
    let t = Instant::now();
    let status = Command::new("bb")
        .args([
            "prove",
            "-b",
            circuit_dir
                .join("target")
                .join(CIRCUIT_JSON)
                .to_str()
                .unwrap(),
            "-w",
            witness.to_str().unwrap(),
            "-k",
            vk_path.to_str().unwrap(), 
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("bb not found: {}", e))?;

    info!("proof generation: {:?}", t.elapsed());

    if !status.success() {
        return Err(format!("bb prove failed (exit {:?})", status.code()));
    }

    let _ = fs::remove_file(&witness);

    Ok(ProveResult {
        proof: fs::read(out_dir.join(PROOF_FILE)).map_err(|e| e.to_string())?,
        vk: fs::read(vk_path).map_err(|e| e.to_string())?,
        public_inputs: fs::read(out_dir.join(PUB_INPUT_FILE)).map_err(|e| e.to_string())?,
        message: message.to_vec(),
        signature: signature.to_vec(), 
    })
}
