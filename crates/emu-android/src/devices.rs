//! Parses Android's own device hardware-profile XML files into [`DeviceProfile`]s.
//!
//! `avdmanager list device`'s plain-text output only carries `id`/`Name`/`OEM`/`Tag` — not the
//! screen/RAM/sensor data `DeviceProfile` needs. That richer data lives in the XML files
//! `avdmanager` itself reads, shipped inside `cmdline-tools`' `lib/sdklib/sdklib.core.jar` at
//! `com/android/sdklib/devices/<name>.xml`. This parser reads those files directly — no process
//! spawn — following the same "parse the real data file, don't shell out and scrape text" choice
//! task `0010` made for the component catalog.
//!
//! Six files exist (verified 2026-09-05 against cmdline-tools `16111833`, revision 23):
//! `devices.xml` (generic legacy profiles + "Small/Medium Phone/Tablet"), `nexus.xml`
//! (Pixel/Nexus phones), `wear.xml`, `tv.xml`, `automotive.xml`, `desktop.xml`. A seventh,
//! `xr.xml` (glasses/XR devices), exists too but is out of scope — `docs/spec.md` never mentions
//! an XR form factor and `FormFactor` has no variant for one; picking it up is a follow-up task if
//! a real need shows up (mirrors how M1 deferred `sdkmanager --list` parsing until it was needed).
//!
//! Source and shape cited against real, trimmed captures in `tests/fixtures/{devices,nexus,wear,
//! tv,automotive,desktop}.xml`. Fixture note: `wear.xml`/`tv.xml`/`automotive.xml` carry an
//! explicit `<d:tag-id>`; `devices.xml`/`nexus.xml`/`desktop.xml` don't — see [`DeviceSource`] for
//! how form factor is assigned in each case.

use emu_core::model::{DeviceProfile, FormFactor, Screen};
use emu_core::{CoreError, Result};

/// Which of the six real device-definition files `xml` came from — determines how
/// [`FormFactor`] is assigned (see the module doc and [`form_factor_for`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSource {
    /// `devices.xml` or `nexus.xml` — generic/legacy profiles and Google's Pixel/Nexus phones.
    /// None of these carry an explicit form-factor tag, so Phone/Tablet/Foldable is inferred from
    /// the device's id/name (documented heuristic — not something `avdmanager` itself emits).
    Handset,
    /// `wear.xml` — every device tagged `android-wear`.
    Wear,
    /// `tv.xml` — every device tagged `android-tv`.
    Tv,
    /// `automotive.xml` — every device tagged `android-automotive` or `android-automotive-playstore`;
    /// `FormFactor` doesn't distinguish the Play Store variant.
    Automotive,
    /// `desktop.xml` — every device tagged `android-desktop`.
    Desktop,
}

/// The real jar-relative resource paths this parser is meant to be fed, paired with the
/// [`DeviceSource`] that determines their form factor. A caller (task `0015`+) locates the
/// installed `sdklib.core.jar` and extracts each entry's bytes.
pub const DEVICE_XML_RESOURCES: &[(&str, DeviceSource)] = &[
    (
        "com/android/sdklib/devices/devices.xml",
        DeviceSource::Handset,
    ),
    (
        "com/android/sdklib/devices/nexus.xml",
        DeviceSource::Handset,
    ),
    ("com/android/sdklib/devices/wear.xml", DeviceSource::Wear),
    ("com/android/sdklib/devices/tv.xml", DeviceSource::Tv),
    (
        "com/android/sdklib/devices/automotive.xml",
        DeviceSource::Automotive,
    ),
    (
        "com/android/sdklib/devices/desktop.xml",
        DeviceSource::Desktop,
    ),
];

/// Parse one device-definition XML file into its [`DeviceProfile`]s.
///
/// A `<d:device>` element missing an id, name, or the hardware/screen/RAM data every real file
/// carries is skipped rather than erroring the whole file — only XML that doesn't parse at all is
/// an error.
///
/// # Errors
///
/// Returns [`CoreError::Parse`] when `xml` isn't valid UTF-8 or isn't well-formed XML.
pub fn parse(xml: &[u8], source: DeviceSource) -> Result<Vec<DeviceProfile>> {
    let text = std::str::from_utf8(xml)
        .map_err(|e| CoreError::parse("device definitions", format!("not valid utf-8: {e}")))?;
    let doc =
        roxmltree::Document::parse(text).map_err(|e| CoreError::parse("device definitions", e))?;

    Ok(doc
        .descendants()
        .filter(|n| n.has_tag_name("device"))
        .filter_map(|node| build_profile(&node, source))
        .collect())
}

fn build_profile(node: &roxmltree::Node<'_, '_>, source: DeviceSource) -> Option<DeviceProfile> {
    let id = text_of(node, "id")?.to_string();
    let display_name = text_of(node, "name")?.to_string();
    let oem = text_of(node, "manufacturer")
        .unwrap_or("")
        .trim()
        .to_string();

    let hardware = node.children().find(|n| n.has_tag_name("hardware"))?;
    let screen_node = hardware.children().find(|n| n.has_tag_name("screen"))?;
    let dims = screen_node
        .children()
        .find(|n| n.has_tag_name("dimensions"))?;
    let width_px = text_of(&dims, "x-dimension")?.parse().ok()?;
    let height_px = text_of(&dims, "y-dimension")?.parse().ok()?;
    let density_dpi = parse_density_dpi(text_of(&screen_node, "pixel-density")?)?;
    let diagonal_in = text_of(&screen_node, "diagonal-length")?.parse().ok()?;
    let default_ram_mb = parse_ram_mb(&hardware)?;
    let sensors = sensors_of(&hardware);
    let form_factor = form_factor_for(source, &id, &display_name);

    Some(DeviceProfile {
        id,
        display_name,
        oem,
        form_factor,
        screen: Screen {
            width_px,
            height_px,
            density_dpi,
            diagonal_in,
        },
        default_ram_mb,
        sensors,
        is_custom: false,
    })
}

/// `Phone`/`Tablet`/`Foldable` for [`DeviceSource::Handset`] entries — inferred from id/name since
/// none of the real files tag these explicitly. Every other source maps 1:1 to its file.
fn form_factor_for(source: DeviceSource, id: &str, name: &str) -> FormFactor {
    match source {
        DeviceSource::Wear => FormFactor::Wear,
        DeviceSource::Tv => FormFactor::Tv,
        DeviceSource::Automotive => FormFactor::Automotive,
        DeviceSource::Desktop => FormFactor::Desktop,
        DeviceSource::Handset => {
            let haystack = format!("{id} {name}").to_lowercase();
            if haystack.contains("tablet") {
                FormFactor::Tablet
            } else if haystack.contains("fold") || haystack.contains("rollable") {
                FormFactor::Foldable
            } else {
                FormFactor::Phone
            }
        }
    }
}

/// `<d:pixel-density>` is either a bare number with a `dpi` suffix (`"420dpi"`) or one of
/// Android's named density buckets. Named-bucket values are `DisplayMetrics.DENSITY_*`
/// constants (<https://developer.android.com/reference/android/util/DisplayMetrics>).
fn parse_density_dpi(raw: &str) -> Option<u32> {
    let raw = raw.trim();
    if let Some(n) = raw.strip_suffix("dpi").and_then(|s| s.parse::<u32>().ok()) {
        return Some(n);
    }
    match raw {
        "ldpi" => Some(120),
        "mdpi" => Some(160),
        "tvdpi" => Some(213),
        "hdpi" => Some(240),
        "xhdpi" => Some(320),
        "xxhdpi" => Some(480),
        "xxxhdpi" => Some(640),
        _ => None,
    }
}

/// `<d:ram unit="GiB|MiB|KiB">N</d:ram>` converted to whole MiB. The real files use all three
/// units (`nexus.xml` GiB, `wear.xml` MiB, `automotive.xml` KiB — see the fixtures).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn parse_ram_mb(hardware: &roxmltree::Node<'_, '_>) -> Option<u32> {
    let ram = hardware.children().find(|n| n.has_tag_name("ram"))?;
    let unit = ram.attribute("unit").unwrap_or("MiB");
    let raw: f64 = ram.text()?.trim().parse().ok()?;
    let mb = match unit {
        "GiB" => raw * 1024.0,
        "KiB" => raw / 1024.0,
        _ => raw,
    };
    Some(mb.round().clamp(1.0, f64::from(u32::MAX)) as u32)
}

/// Whitespace-separated sensor names from `<d:sensors>` (one per line in the real files, but the
/// schema doesn't require that — split on any whitespace).
fn sensors_of(hardware: &roxmltree::Node<'_, '_>) -> Vec<String> {
    hardware
        .children()
        .find(|n| n.has_tag_name("sensors"))
        .and_then(|n| n.text())
        .map(|text| text.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

/// Text content of the first direct child named `tag`, if any.
fn text_of<'a>(node: &roxmltree::Node<'a, 'a>, tag: &str) -> Option<&'a str> {
    node.children().find(|n| n.has_tag_name(tag))?.text()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEVICES: &[u8] = include_bytes!("../tests/fixtures/devices.xml");
    const NEXUS: &[u8] = include_bytes!("../tests/fixtures/nexus.xml");
    const WEAR: &[u8] = include_bytes!("../tests/fixtures/wear.xml");
    const TV: &[u8] = include_bytes!("../tests/fixtures/tv.xml");
    const AUTOMOTIVE: &[u8] = include_bytes!("../tests/fixtures/automotive.xml");
    const DESKTOP: &[u8] = include_bytes!("../tests/fixtures/desktop.xml");

    #[test]
    fn generic_file_infers_phone_tablet_and_foldable() {
        let got = parse(DEVICES, DeviceSource::Handset).expect("parse");
        assert_eq!(got.len(), 3);

        let phone = got.iter().find(|d| d.id == "medium_phone").expect("phone");
        assert_eq!(phone.form_factor, FormFactor::Phone);
        assert_eq!(phone.oem, "Generic");
        assert_eq!(phone.default_ram_mb, 8192); // 8 GiB
        assert!(phone.sensors.contains(&"Accelerometer".to_string()));
        assert_eq!(phone.screen.width_px, 1080);
        assert_eq!(phone.screen.height_px, 2400);
        assert_eq!(phone.screen.density_dpi, 420);

        let tablet = got
            .iter()
            .find(|d| d.id == "medium_tablet")
            .expect("tablet");
        assert_eq!(tablet.form_factor, FormFactor::Tablet);

        let foldable = got
            .iter()
            .find(|d| d.id == "7.6in Foldable")
            .expect("foldable");
        assert_eq!(foldable.form_factor, FormFactor::Foldable);
    }

    #[test]
    fn nexus_file_is_a_phone() {
        let got = parse(NEXUS, DeviceSource::Handset).expect("parse");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "pixel_6");
        assert_eq!(got[0].display_name, "Pixel 6");
        assert_eq!(got[0].oem, "Google");
        assert_eq!(got[0].form_factor, FormFactor::Phone);
        assert_eq!(got[0].default_ram_mb, 8192);
    }

    #[test]
    fn wear_tv_automotive_desktop_map_by_source() {
        let wear = parse(WEAR, DeviceSource::Wear).expect("parse");
        assert_eq!(wear.len(), 1);
        assert_eq!(wear[0].form_factor, FormFactor::Wear);
        assert_eq!(wear[0].default_ram_mb, 512); // MiB unit, no conversion

        let tv = parse(TV, DeviceSource::Tv).expect("parse");
        assert_eq!(tv.len(), 1);
        assert_eq!(tv[0].form_factor, FormFactor::Tv);

        let automotive = parse(AUTOMOTIVE, DeviceSource::Automotive).expect("parse");
        assert_eq!(automotive.len(), 2);
        assert!(automotive
            .iter()
            .all(|d| d.form_factor == FormFactor::Automotive));
        // automotive.xml's RAM is in KiB — confirms the third unit conversion path.
        let landscape_1024 = automotive
            .iter()
            .find(|d| d.id == "automotive_1024p_landscape")
            .expect("device");
        assert_eq!(landscape_1024.default_ram_mb, 3_774_492 / 1024);

        let desktop = parse(DESKTOP, DeviceSource::Desktop).expect("parse");
        assert_eq!(desktop.len(), 1);
        assert_eq!(desktop[0].form_factor, FormFactor::Desktop);
    }

    #[test]
    fn density_bucket_names_resolve_to_real_dpi_values() {
        assert_eq!(parse_density_dpi("420dpi"), Some(420));
        assert_eq!(parse_density_dpi("mdpi"), Some(160));
        assert_eq!(parse_density_dpi("xxhdpi"), Some(480));
        assert_eq!(parse_density_dpi("not-a-density"), None);
    }

    #[test]
    fn empty_document_is_an_empty_list_not_an_error() {
        let empty = b"<d:devices xmlns:d=\"http://schemas.android.com/sdk/devices/7\"/>";
        assert_eq!(parse(empty, DeviceSource::Handset).expect("parse"), vec![]);
    }

    #[test]
    fn malformed_xml_is_a_parse_error_not_a_panic() {
        let err = parse(b"<not-xml", DeviceSource::Handset).unwrap_err();
        assert_eq!(err.code(), "parse_error");
    }
}
