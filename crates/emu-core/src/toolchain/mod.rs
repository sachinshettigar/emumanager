//! Toolchain manager (milestone M1) — turns an empty data dir into a working, licensed
//! `sdkmanager` with `platform-tools` and `emulator` installed, reusing whatever the machine
//! already has instead of re-downloading it.
//!
//! - [`installed_state`] — what's on disk right now, app-managed *and* any existing system SDK.
//! - [`bootstrap`] — fetch + unpack `cmdline-tools`, accept licenses, install the rest.

pub mod bootstrap;
pub mod installed_state;

pub use bootstrap::{bootstrap, BootstrapPorts};
pub use installed_state::{binary_path, scan, ComponentLocation, InstalledState, SdkSource};
