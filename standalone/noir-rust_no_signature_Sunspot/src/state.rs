use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct AppState {
    pub semaphore:    Arc<Semaphore>,
    pub project_root: PathBuf,
    pub vk_path:      PathBuf,
}
