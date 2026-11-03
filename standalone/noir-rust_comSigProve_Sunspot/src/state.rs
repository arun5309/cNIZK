use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct AppState {
    pub signing_key:      [u8; 32],
    pub issuer_pub_key_x: [u8; 32],
    pub issuer_pub_key_y: [u8; 32],
    pub semaphore:        Arc<Semaphore>,
    pub project_root:     PathBuf,
    pub vk_path:          PathBuf,
}
