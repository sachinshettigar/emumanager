//! `emu-core` — domain logic and orchestration for EmuManager.
//!
//! This crate holds all business logic and defines the port traits for every side effect
//! (process spawning, downloads, host probing). It has **no `tauri` dependency** so it stays
//! testable with `cargo test` alone — see `AGENTS.md` §6 and `docs/architecture.md` §2.
//!
//! Models and ports land in task `0002`; this is the skeleton.

#![forbid(unsafe_code)]

/// Errors returned by domain operations.
///
/// Real variants land with the models in task `0002`. Each variant maps to a stable
/// [`CoreError::code`] string that the IPC layer surfaces to the frontend.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CoreError {
    /// A code path that is scaffolded but not yet implemented.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

impl CoreError {
    /// Stable machine-readable code for the IPC error contract.
    ///
    /// The frontend switches on this string; keep the values stable across releases.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::NotImplemented(_) => "not_implemented",
        }
    }
}

/// Crate result alias.
pub type Result<T, E = CoreError> = core::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::CoreError;

    #[test]
    fn error_code_is_stable() {
        assert_eq!(CoreError::NotImplemented("x").code(), "not_implemented");
    }
}
