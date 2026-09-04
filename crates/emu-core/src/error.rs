//! [`CoreError`] — the single error type crossing the domain boundary.
//!
//! Every variant maps to a stable [`CoreError::code`] string. The IPC layer (task `0004`)
//! forwards that code to the frontend, which switches on it for user-facing messaging, so the
//! string values are part of the public contract and must not change casually.

use std::fmt;

/// Errors returned by domain operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CoreError {
    /// A code path that is scaffolded but not yet implemented.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),

    /// Failed to parse an external tool's output or a user-supplied string.
    #[error("could not parse {what}: {detail}")]
    Parse {
        /// What was being parsed (e.g. `"image coordinate"`).
        what: &'static str,
        /// Human-readable detail about why it failed.
        detail: String,
    },

    /// A required resource (AVD, image, file, device profile) was not found.
    #[error("{what} not found: {name}")]
    NotFound {
        /// The kind of resource (e.g. `"emulator"`).
        what: &'static str,
        /// The identifier that was looked up.
        name: String,
    },

    /// A value was well-formed but not acceptable in context.
    #[error("invalid {what}: {detail}")]
    Invalid {
        /// What was invalid (e.g. `"hardware spec"`).
        what: &'static str,
        /// Why it was rejected and, ideally, what to do instead.
        detail: String,
    },

    /// The host or configuration cannot support the requested operation.
    #[error("unsupported: {0}")]
    Unsupported(String),

    /// An external process exited non-zero.
    #[error("`{program}` exited with {code}: {stderr}")]
    Process {
        /// The program that was run (e.g. `"avdmanager"`).
        program: String,
        /// Its exit code.
        code: i32,
        /// Captured stderr, trimmed.
        stderr: String,
    },

    /// A download failed or failed verification.
    #[error("download of {url} failed: {detail}")]
    Download {
        /// The URL that was being fetched.
        url: String,
        /// What went wrong (transport error, checksum mismatch, …).
        detail: String,
    },

    /// A filesystem operation failed.
    #[error("filesystem error at {path}: {detail}")]
    Fs {
        /// The path involved.
        path: String,
        /// The underlying error message.
        detail: String,
    },

    /// The operation was cancelled by the user or a shutdown.
    #[error("operation cancelled")]
    Cancelled,
}

impl CoreError {
    /// Stable machine-readable code for the IPC error contract.
    ///
    /// The frontend switches on this string; keep the values stable across releases.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::NotImplemented(_) => "not_implemented",
            CoreError::Parse { .. } => "parse_error",
            CoreError::NotFound { .. } => "not_found",
            CoreError::Invalid { .. } => "invalid",
            CoreError::Unsupported(_) => "unsupported",
            CoreError::Process { .. } => "process_failed",
            CoreError::Download { .. } => "download_failed",
            CoreError::Fs { .. } => "fs_error",
            CoreError::Cancelled => "cancelled",
        }
    }

    /// Build a [`CoreError::Parse`] from any `Display` detail.
    pub fn parse(what: &'static str, detail: impl fmt::Display) -> Self {
        CoreError::Parse {
            what,
            detail: detail.to_string(),
        }
    }

    /// Build a [`CoreError::Invalid`] from any `Display` detail.
    pub fn invalid(what: &'static str, detail: impl fmt::Display) -> Self {
        CoreError::Invalid {
            what,
            detail: detail.to_string(),
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
        // These strings are a published contract. Changing one is a breaking change.
        assert_eq!(CoreError::NotImplemented("x").code(), "not_implemented");
        assert_eq!(
            CoreError::parse("image coordinate", "bad").code(),
            "parse_error",
        );
        assert_eq!(
            CoreError::NotFound {
                what: "emulator",
                name: "pixel".into(),
            }
            .code(),
            "not_found",
        );
        assert_eq!(CoreError::invalid("hardware", "too big").code(), "invalid");
        assert_eq!(
            CoreError::Unsupported("no kvm".into()).code(),
            "unsupported"
        );
        assert_eq!(
            CoreError::Process {
                program: "adb".into(),
                code: 1,
                stderr: "boom".into(),
            }
            .code(),
            "process_failed",
        );
        assert_eq!(
            CoreError::Download {
                url: "https://x".into(),
                detail: "checksum".into(),
            }
            .code(),
            "download_failed",
        );
        assert_eq!(
            CoreError::Fs {
                path: "/tmp/x".into(),
                detail: "eperm".into(),
            }
            .code(),
            "fs_error",
        );
        assert_eq!(CoreError::Cancelled.code(), "cancelled");
    }

    #[test]
    fn display_includes_context() {
        let e = CoreError::Process {
            program: "avdmanager".into(),
            code: 2,
            stderr: "no such device".into(),
        };
        let s = e.to_string();
        assert!(s.contains("avdmanager"));
        assert!(s.contains("no such device"));
    }
}
