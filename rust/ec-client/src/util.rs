use std::time::{SystemTime, UNIX_EPOCH};

/// Minimal LCG for decoration color randomization (same constants as mapgen).
pub struct Lcg {
    state: u64,
}

impl Lcg {
    pub fn from_time() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xEC15);
        Self {
            state: seed.wrapping_mul(6364136223846793005).wrapping_add(1),
        }
    }

    pub fn next_usize(&mut self) -> usize {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as usize
    }
}
