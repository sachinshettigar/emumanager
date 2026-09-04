//! The local SQLite registry — metadata about emulators, images, profiles, jobs.
//!
//! [`Registry::open`] creates and migrates a database in the app data dir. The
//! minimal schema lives in `migrations/0001_init.sql`; the full schema (columns,
//! indexes, the job/event tables) lands in M3. Queries elsewhere in the crate
//! run against [`Registry::pool`].

mod open;

pub use open::Registry;
