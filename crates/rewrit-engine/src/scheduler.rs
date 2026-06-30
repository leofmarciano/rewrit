#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerConfig {
    pub max_parallel: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self { max_parallel: 1 }
    }
}
