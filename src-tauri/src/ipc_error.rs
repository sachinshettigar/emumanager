//! The single error type every `#[tauri::command]` returns.
//!
//! Commands never leak `anyhow` or provider-specific error types across the IPC
//! seam. They map into [`IpcError`], whose `code` is the stable machine string
//! from [`emu_core::CoreError::code`] — the frontend switches on `code`, shows
//! `message`, and treats `details` as optional structured context.

use serde::Serialize;

/// Serializable error surfaced to the frontend across the typed IPC seam.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    /// Stable machine-readable code, e.g. `not_found`, `process_failed`.
    /// Sourced from [`emu_core::CoreError::code`] when the origin is a core error.
    pub code: String,
    /// Human-facing description of what went wrong and, where possible, what to do.
    pub message: String,
    /// Optional structured context (paths, exit codes, offending values).
    /// Exported to TypeScript as `unknown` — `serde_json::Value` carries
    /// arbitrary JSON and specta will not emit its BigInt-bearing `Number`.
    #[specta(type = specta_typescript::Unknown)]
    pub details: Option<serde_json::Value>,
}

impl IpcError {
    /// Build an error from a code and message, with no details.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Attach structured context.
    #[must_use]
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for IpcError {}

impl From<emu_core::CoreError> for IpcError {
    fn from(err: emu_core::CoreError) -> Self {
        Self::new(err.code(), err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::IpcError;
    use emu_core::CoreError;

    #[test]
    fn maps_core_error_code_and_message() {
        let core = CoreError::NotFound {
            what: "avd",
            name: "Pixel_7".into(),
        };
        let ipc: IpcError = core.into();
        assert_eq!(ipc.code, "not_found");
        assert!(
            ipc.message.contains("Pixel_7"),
            "message keeps context: {}",
            ipc.message
        );
        assert!(ipc.details.is_none());
    }

    #[test]
    fn every_core_code_survives_the_mapping() {
        // A representative of each `CoreError` shape; `code()` is the pinned string.
        let cases: [(CoreError, &str); 4] = [
            (CoreError::NotImplemented("x"), "not_implemented"),
            (CoreError::Unsupported("nested virt".into()), "unsupported"),
            (
                CoreError::Process {
                    program: "adb".into(),
                    code: 1,
                    stderr: "boom".into(),
                },
                "process_failed",
            ),
            (CoreError::Cancelled, "cancelled"),
        ];
        for (core, want) in cases {
            assert_eq!(IpcError::from(core).code, want);
        }
    }

    #[test]
    fn serializes_camel_case_with_null_details() {
        let json = serde_json::to_value(IpcError::new("invalid", "bad input")).unwrap();
        assert_eq!(json["code"], "invalid");
        assert_eq!(json["message"], "bad input");
        assert!(json.get("details").is_some_and(serde_json::Value::is_null));
    }
}
