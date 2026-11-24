use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RunnerWorkspace {
    pub root: PathBuf,
}

impl Default for RunnerWorkspace {
    fn default() -> Self {
        Self {
            root: std::env::temp_dir().join("forge-runner"),
        }
    }
}