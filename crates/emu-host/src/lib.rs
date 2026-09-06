//! `emu-host` — implements the `HostProbe` port: detects CPU virtualization, the available
//! accelerator (KVM / WHPX / AEHD / HVF) and its status, free disk and memory, and produces a
//! `HostReport` with a launch verdict and a list of fixes.
//!
//! - [`signals`] — [`HostSignals`], the OS-independent input.
//! - [`report::build_report`] — the pure signals → [`HostReport`](emu_core::model::host::HostReport)
//!   derivation (verdict + fixes). Fully unit-tested.
//! - [`probe`] — [`NativeHostProbe`], the real per-OS gatherer behind `#[cfg(target_os)]`.

#![forbid(unsafe_code)]

pub mod probe;
pub mod report;
pub mod signals;

pub use emu_core::{CoreError, Result};
pub use probe::NativeHostProbe;
pub use report::build_report;
pub use signals::{AccelSignal, HostSignals};
