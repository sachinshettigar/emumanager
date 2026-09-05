//! The local SQLite registry — metadata about emulators, images, profiles, jobs.
//!
//! [`Registry::open`] creates and migrates a database in the app data dir. The minimal bootstrap
//! schema is `migrations/0001_init.sql`; the full M3 schema (task `0018`) is
//! `migrations/0002_registry_m3.sql`. [`Registry`]'s methods take and return typed [`EmulatorRow`]
//! records rather than column tuples; compound fields are stored as JSON text so struct changes
//! don't force a migration. Queries elsewhere in the crate run against [`Registry::pool`].

mod open;
mod row;

pub use open::Registry;
pub use row::{EmulatorRow, HostSnapshotRow};
