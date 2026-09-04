//! `emu-host` — implements the `HostProbe` port: detects CPU virtualization, the available
//! accelerator (KVM / WHPX / AEHD / HVF) and its status, free disk and memory, and produces a
//! `HostReport` with a launch verdict and a list of fixes.
//!
//! Implementations land in task series M5; this is the skeleton.

#![forbid(unsafe_code)]

pub use emu_core::{CoreError, Result};
