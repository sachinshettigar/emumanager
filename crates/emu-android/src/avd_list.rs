//! Parses `avdmanager list avd` plain-text output into [`AvdEntry`]s — the on-disk AVD inventory
//! `AndroidProvider::reconcile` compares the registry against.
//!
//! Output shape cited against a real capture (`tests/fixtures/avdmanager-list-avd.txt`), taken
//! 2026-09-06 from `cmdline-tools` build `16111833` (rev 23) on macOS. Two normalizations vs. the
//! raw capture, neither touching what this parser reads: the machine-specific `Path:` prefix was
//! rewritten to `/home/user/.android/avd`, and the trailing space on the (empty) `Target:` line
//! was dropped.
//!
//! Real quirks this surfaced:
//! - A loadable entry is a block of `Name:` / `Device:` / `Path:` / `Target:` / `Based on: … Tag/ABI:`
//!   / `Sdcard:` lines; blocks are separated by a line of exactly nine dashes (`---------`).
//! - `Target:` is printed **empty** even with the matching `platforms;android-NN` installed — the
//!   android-version text lives on the indented `Based on:` continuation line instead.
//! - Broken AVDs come after a `The following Android Virtual Devices could not be loaded:` header
//!   and carry only `Name:` / `Path:` / `Error:`. The `-c` (compact) form omits them entirely,
//!   which is why `reconcile` parses this verbose form.
//! - `avdmanager list avd` with no AVDs prints just the `Available Android Virtual Devices:` header.

use std::path::PathBuf;

/// One entry from `avdmanager list avd`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AvdEntry {
    /// The AVD name — the `-n` value and the on-disk `<name>.avd` directory stem.
    pub name: String,
    /// The `Device:` line verbatim, e.g. `pixel_6 (Google)`. `None` for a broken entry.
    pub device: Option<String>,
    /// The `Path:` line — the AVD's `.avd` directory.
    pub path: Option<PathBuf>,
    /// `<tag>/<abi>` from the `Based on: … Tag/ABI: <tag>/<abi>` line, e.g. `default/x86_64`.
    pub tag_abi: Option<String>,
    /// The `Based on:` android-version text, e.g. `Android 7.0 ("Nougat")`.
    pub based_on: Option<String>,
    /// `false` when the entry was listed under "could not be loaded".
    pub loadable: bool,
    /// The `Error:` text for a non-loadable entry.
    pub error: Option<String>,
}

const COULD_NOT_LOAD_HEADER: &str = "The following Android Virtual Devices could not be loaded:";

/// Parse the full output of `avdmanager list avd` (not `-c`). Never fails: unrecognized lines are
/// ignored, so a future field or a reordered block still yields every `Name:` it can see.
#[must_use]
pub fn parse_avdmanager_list_avd(output: &str) -> Vec<AvdEntry> {
    let mut entries = Vec::new();
    let mut current: Option<AvdEntry> = None;
    let mut loadable = true;

    for raw in output.lines() {
        let line = raw.trim();

        if line == COULD_NOT_LOAD_HEADER {
            push(&mut entries, current.take());
            loadable = false;
            continue;
        }
        let is_separator = !line.is_empty() && line.bytes().all(|b| b == b'-');
        if line.is_empty() || is_separator {
            // Blank line or the `---------` block separator: end the current block.
            push(&mut entries, current.take());
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();

        match key.trim() {
            "Name" => {
                push(&mut entries, current.take());
                current = Some(AvdEntry {
                    name: value.to_string(),
                    loadable,
                    ..AvdEntry::default()
                });
            }
            "Device" => set(&mut current, |e| e.device = Some(value.to_string())),
            "Path" => set(&mut current, |e| e.path = Some(PathBuf::from(value))),
            "Error" => set(&mut current, |e| e.error = Some(value.to_string())),
            "Based on" => {
                // `Android 7.0 ("Nougat") Tag/ABI: default/x86_64`
                if let Some((based_on, tag_abi)) = value.split_once("Tag/ABI:") {
                    set(&mut current, |e| {
                        e.based_on = Some(based_on.trim().to_string());
                        e.tag_abi = Some(tag_abi.trim().to_string());
                    });
                } else {
                    set(&mut current, |e| e.based_on = Some(value.to_string()));
                }
            }
            _ => {} // Target:, Sdcard:, Snapshot:, … — not needed by reconcile.
        }
    }
    push(&mut entries, current.take());
    entries
}

fn push(entries: &mut Vec<AvdEntry>, entry: Option<AvdEntry>) {
    if let Some(entry) = entry {
        if !entry.name.is_empty() {
            entries.push(entry);
        }
    }
}

fn set(current: &mut Option<AvdEntry>, f: impl FnOnce(&mut AvdEntry)) {
    if let Some(entry) = current.as_mut() {
        f(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL: &str = include_str!("../tests/fixtures/avdmanager-list-avd.txt");

    #[test]
    fn parses_the_real_capture_into_two_loadable_and_one_broken() {
        let entries = parse_avdmanager_list_avd(REAL);
        assert_eq!(entries.len(), 3);

        let pixel = &entries[0];
        assert_eq!(pixel.name, "Pixel6_API24");
        assert_eq!(pixel.device.as_deref(), Some("pixel_6 (Google)"));
        assert_eq!(
            pixel.path,
            Some(PathBuf::from("/home/user/.android/avd/Pixel6_API24.avd"))
        );
        assert_eq!(pixel.tag_abi.as_deref(), Some("default/x86_64"));
        assert_eq!(pixel.based_on.as_deref(), Some(r#"Android 7.0 ("Nougat")"#));
        assert!(pixel.loadable);
        assert!(pixel.error.is_none());

        assert_eq!(entries[1].name, "Wear_Small_API24");
        assert!(entries[1].loadable);

        let broken = &entries[2];
        assert_eq!(broken.name, "Broken_API99");
        assert!(!broken.loadable);
        assert_eq!(
            broken.error.as_deref(),
            Some("Missing system image android-99/default/x86_64.")
        );
        assert!(broken.device.is_none());
    }

    #[test]
    fn empty_listing_is_no_entries() {
        assert!(parse_avdmanager_list_avd("Available Android Virtual Devices:\n").is_empty());
        assert!(parse_avdmanager_list_avd("").is_empty());
    }

    #[test]
    fn a_lone_broken_entry_is_still_found() {
        let out = "Available Android Virtual Devices:\n\n\
                   The following Android Virtual Devices could not be loaded:\n\
                   \x20   Name: Ghost\n\
                   \x20   Path: /x/Ghost.avd\n\
                   \x20  Error: Missing system image android-40/default/x86_64.\n";
        let entries = parse_avdmanager_list_avd(out);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Ghost");
        assert!(!entries[0].loadable);
    }
}
