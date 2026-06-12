//! Sandbox knobs for test runs.
//!
//! For now we enforce wall-clock timeout and an empty env. Real
//! resource limits (cgroups / rlimit / seccomp) belong to a future
//! `LinuxSandbox` backend; on macOS we'd lean on `sandbox-exec`.

#[derive(Debug, Clone, Copy)]
pub struct SandboxLimits {
    pub wall_seconds: u64,
    pub cpu_seconds: u64,
    pub memory_mb: u64,
    pub allow_network: bool,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            wall_seconds: 60,
            cpu_seconds: 30,
            memory_mb: 512,
            allow_network: false,
        }
    }
}
