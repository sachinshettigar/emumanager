//! The M5 definition-of-done's second half: `emu-helper <cmd>` always prints the agreed
//! single-line JSON shape with a `status` from the allowed set. Runs each subcommand as a real
//! subprocess.

use std::process::Command;

const ALLOWED_STATUS: &[&str] = &["ok", "failed", "notApplicable", "needsReboot"];

fn run(sub: &str) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_emu-helper"))
        .arg(sub)
        .output()
        .expect("spawn emu-helper");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{sub}: stdout is not JSON ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

#[test]
fn every_subcommand_prints_the_agreed_json_shape() {
    for sub in ["check", "enable-whpx", "enable-aehd", "add-kvm-group"] {
        let v = run(sub);
        assert_eq!(
            v["command"], sub,
            "{sub}: command field echoes the subcommand"
        );

        let status = v["status"].as_str().unwrap_or_default();
        assert!(
            ALLOWED_STATUS.contains(&status),
            "{sub}: status {status:?} not in {ALLOWED_STATUS:?}"
        );
        assert!(
            v["message"].as_str().is_some_and(|m| !m.is_empty()),
            "{sub}: message must be a non-empty string"
        );
        assert!(
            v["needsReboot"].is_boolean(),
            "{sub}: needsReboot must be a bool"
        );
    }
}

#[test]
fn a_failed_status_exits_nonzero_and_everything_else_exits_zero() {
    for sub in ["check", "enable-whpx", "enable-aehd", "add-kvm-group"] {
        let out = Command::new(env!("CARGO_BIN_EXE_emu-helper"))
            .arg(sub)
            .output()
            .expect("spawn");
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
        let failed = v["status"] == "failed";
        assert_eq!(
            out.status.success(),
            !failed,
            "{sub}: exit code must match status (failed => nonzero)"
        );
    }
}
