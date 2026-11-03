use std::{fs, path::PathBuf};
use tracing::error;

pub struct DirCleanup(pub PathBuf);
impl Drop for DirCleanup {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_dir_all(&self.0) {
            error!("failed to cleanup dir {:?}: {}", self.0, e);
        }
    }
}

pub fn bytes_to_toml_array(v: &[u8]) -> String {
    format!(
        "[{}]",
        v.iter()
            .map(|x| format!("\"0x{:02x}\"", x))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub fn field_to_toml(hash: &[u8; 32]) -> String {
    format!("\"0x{}\"", hex::encode(hash))
}

pub fn ints_to_toml(v: &[u64]) -> String {
    format!(
        "[{}]",
        v.iter()
            .map(|x| format!("\"{}\"", x))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
