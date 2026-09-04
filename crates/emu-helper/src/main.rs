//! `emu-helper` — a tiny standalone binary the app invokes **with OS elevation** to perform the
//! scriptable parts of accelerator setup. It shares no state with the app beyond argv and a
//! single JSON object on stdout.
//!
//! Every subcommand currently reports `not_implemented`; the real logic lands in milestone M5
//! (`docs/architecture.md` §5, `docs/playbooks/debug-emulator-boot.md`).

#![forbid(unsafe_code)]

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
enum Command {
    /// Report what the host has and what this helper could change.
    Check,
    /// Enable the Windows Hypervisor Platform feature.
    EnableWhpx,
    /// Install and enable AEHD (the HAXM successor) on Windows.
    EnableAehd,
    /// Add the current user to the `kvm` group on Linux.
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

#[derive(Serialize)]
struct Outcome {
    command: &'static str,
    status: &'static str,
    message: &'static str,
    needs_reboot: bool,
}

fn main() {
    let cli = Cli::parse();
    let outcome = Outcome {
        command: cli.command.slug(),
        status: "not_implemented",
        message: "emu-helper is scaffolded; accelerator setup lands in milestone M5.",
        needs_reboot: false,
    };
    // Structured, single-line output — the app parses this from stdout.
    println!(
        "{}",
        serde_json::to_string(&outcome).expect("Outcome serializes to JSON")
    );
}
