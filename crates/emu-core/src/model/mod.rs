//! Domain entities — see `docs/context/domain-model.md`.
//!
//! Everything here is plain data: `#[derive(Serialize, Deserialize)]`, no methods with side
//! effects. `specta::Type` is added crate-wide in task `0004` when the IPC layer pins the
//! matching `specta` / `tauri-specta` versions.
//!
//! Field naming: Rust `snake_case` in storage, `camelCase` on the wire. Structs that cross IPC
//! carry `#[serde(rename_all = "camelCase")]`.

pub mod device;
pub mod emulator;
pub mod host;
pub mod image;
pub mod job;
pub mod plan;
pub mod profile;

pub use device::{DeviceProfile, FormFactor, Screen};
pub use emulator::{Emulator, EmulatorId, EmulatorSource, Graphics, Hardware, LiveState, RunState};
pub use host::{
    Accelerator, AcceleratorKind, AcceleratorStatus, Fix, HostReport, Verdict, Virtualization,
};
pub use image::{Abi, ImageCoord, ImageFilter, ImageType, SystemImage};
pub use job::{Job, JobId, JobKind, JobState};
pub use plan::{CreateSpec, Plan, Requirement, RequirementKind, RequirementStatus};
pub use profile::{
    CustomDevice, EmuProfile, ProfileDevice, ProfileGraphics, ProfileHardware, ProfileImage,
    ProfileImageType, ProfileSeed, SeedSettings, SCHEMA_VERSION,
};
