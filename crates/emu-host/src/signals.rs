//! [`HostSignals`] — the OS-independent input to [`build_report`](crate::build_report).
//!
//! The native probe ([`crate::probe`]) fills this in per OS; unit tests fabricate it, so the
//! verdict/fixes derivation never has to touch real hardware.

use emu_core::model::host::Virtualization;

/// Raw host facts, gathered once, in a form the pure [`build_report`](crate::build_report) can
/// reason about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostSignals {
    /// `"linux"` / `"windows"` / `"macos"` (from `std::env::consts::OS`).
    pub os: String,
    /// `"x86_64"` / `"aarch64"` (from `std::env::consts::ARCH`).
    pub arch: String,
    /// Hardware-virtualization availability.
    pub virtualization: Virtualization,
    /// What the OS-specific accelerator probe found (KVM / HVF / WHPX / AEHD).
    pub accel: AccelSignal,
    /// Total system RAM in bytes; `0` when it couldn't be read.
    pub ram_bytes: u64,
    /// Free space on the data-directory volume in bytes; `0` when it couldn't be read.
    pub disk_free_bytes: u64,
}

/// Outcome of the per-OS accelerator probe, before it's mapped onto
/// [`AcceleratorStatus`](emu_core::model::host::AcceleratorStatus).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccelSignal {
    /// Present and usable right now.
    Ready,
    /// Present, but the current user can't use it (Linux: not in the `kvm` group).
    NoPermission,
    /// The platform supports it but the driver / feature isn't installed.
    Missing,
    /// Installed but switched off (a stopped service, a disabled Windows feature).
    Disabled,
    /// Couldn't determine — treated as a soft unknown, never a hard block.
    Unknown,
}
