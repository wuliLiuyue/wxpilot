#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapDecision {
    pub allowed: bool,
    pub reason: Option<String>,
}

impl BootstrapDecision {
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reason: None,
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
        }
    }
}

pub trait BootstrapGuard {
    fn check(&self) -> BootstrapDecision;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AllowAllBootstrapGuard;

impl BootstrapGuard for AllowAllBootstrapGuard {
    fn check(&self) -> BootstrapDecision {
        BootstrapDecision::allow()
    }
}

pub fn run_bootstrap_guard(guard: &impl BootstrapGuard) -> BootstrapDecision {
    guard.check()
}
