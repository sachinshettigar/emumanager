//! `emu-helper` — a tiny standalone binary the app invokes **with OS elevation** to perform the
//! scriptable parts of accelerator setup. It shares no state with the app beyond argv and a
//! single JSON object on stdout: `{ "command", "status", "message", "needsReboot" }`.
//!
//! `status` is one of `ok` / `failed` / `notApplicable` / `needsReboot`. Exit code is `0` for
//! everything except `failed` (which exits `1`), so a caller can branch on either.
//!
//! It runs as root/Administrator, so anything that needs the *invoking* user reads `$SUDO_USER`
//! (Linux) rather than `$USER`.

#![forbid(unsafe_code)]

mod actions;

use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "emu-helper",
    about = "Elevated helper for EmuManager. Run via the app, not by hand.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Copy)]
pub enum Command {
    /// Report what the host has and what this helper could change (no mutation).
    Check,
    /// Enable the Windows Hypervisor Platform feature.
    EnableWhpx,
    /// Install and enable AEHD (the HAXM successor) on Windows.
    EnableAehd,
    /// Add the invoking user to the `kvm` group on Linux.
    AddKvmGroup,
}

impl Command {
    fn slug(self) -> &'static str {
        match self {
            Command::Check => "check",
            Command::EnableWhpx => "enable-whpx",
            Command::EnableAehd => "enable-aehd",
            Command::AddKvmGroup => "add-kvm-group",
        }
    }
}

/// The single JSON object printed to stdout.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub command: &'static str,
    /// `ok` | `failed` | `notApplicable` | `needsReboot`.
    pub status: &'static str,
    pub message: String,
    pub needs_reboot: bool,
}

impl Outcome {
    pub fn new(command: Command, status: &'static str, message: impl Into<String>) -> Self {
        Self {
            command: command.slug(),
            status,
            message: message.into(),
            needs_reboot: status == "needsReboot",
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let outcome = match cli.command {
        Command::Check => actions::check(cli.command),
        Command::EnableWhpx => actions::enable_whpx(cli.command),
        Command::EnableAehd => actions::enable_aehd(cli.command),
        Command::AddKvmGroup => actions::add_kvm_group(cli.command),
    };

    println!(
        "{}",
        serde_json::to_string(&outcome).expect("Outcome serializes to JSON")
    );
    std::process::exit(i32::from(outcome.status == "failed"));
}
