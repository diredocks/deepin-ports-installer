use serde::{Deserialize, Serialize};

/// Progress counter (`done` / `total` bytes) for streaming operations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Amount {
    pub done: u64,
    pub total: Option<u64>,
}

impl Amount {
    pub const fn new(done: u64, total: Option<u64>) -> Self {
        Self { done, total }
    }

    pub fn percent(self) -> Option<u64> {
        self.total
            .filter(|total| *total > 0)
            .map(|total| self.done.saturating_mul(100) / total)
    }
}
