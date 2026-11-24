#[derive(Debug, Clone, Copy)]
pub struct ConcurrencyPolicy {
    pub max_concurrent_jobs: u32,
}

impl Default for ConcurrencyPolicy {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: 1,
        }
    }
}