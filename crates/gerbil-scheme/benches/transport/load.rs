//! Offered work is independent of runtime worker and in-flight budgets.

#[derive(Clone, Copy)]
pub(super) struct Load {
    pub(super) requests: usize,
    pub(super) bytes: usize,
}

impl Load {
    const fn new(requests: usize, bytes: usize) -> Self {
        Self { requests, bytes }
    }
}

impl std::fmt::Display for Load {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(output, "requests={}_bytes={}", self.requests, self.bytes)
    }
}

// Explicit qualification coordinates, not production policy thresholds.
pub(super) const LOADS: [Load; 12] = [
    Load::new(1_000, 0),
    Load::new(1_000, 16),
    Load::new(1_000, 8192),
    Load::new(1_000, 65536),
    Load::new(10_000, 0),
    Load::new(10_000, 16),
    Load::new(10_000, 8192),
    Load::new(10_000, 65536),
    Load::new(100_000, 0),
    Load::new(100_000, 16),
    Load::new(100_000, 8192),
    Load::new(100_000, 65536),
];
