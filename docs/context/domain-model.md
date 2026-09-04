# Domain model

The core entities and their relationships. This is intent, not a schema dump — the authoritative
shapes are the `#[derive(specta::Type)]` structs in `emu-core` and the `migrations/`.

## Entities

### DeviceProfile
- `id` (e.g. `pixel_6`), `display_name`, `oem`, `form_factor` (phone/tablet/foldable/wear/tv/auto)
- `screen` { width_px, height_px, density_dpi, diagonal_in }
- `default_ram_mb`, `sensors[]`, `is_custom`
- Source: `avdmanager list device` for Google's; user-created for custom.

### SystemImage
- `coord` { api: u32, image_type: ImageType, abi: Abi }  ← the identity
- `android_version` (e.g. "14"), `revision`, `download_size_bytes`, `installed: bool`
- Source: `sdkmanager --list` / repository XML; `installed` from the data dir.

### Emulator  (our tracked instance = an AVD + metadata)
- `id` (our ULID), `avd_name` (the on-disk AVD name), `display_name`
- `device_profile_id`, `image_coord`
- `hardware` { ram_mb, storage_mb, graphics: Graphics, snapshots: bool, cold_boot: bool, device_frame: bool, ... }
- `source`: `Manual` | `FromProfile { profile_id }` | `Imported { profile_id, origin_label }`
- `created_at`, `updated_at`, `tags[]`, `notes`
- **Live** (not persisted, from reconcile): `state: Stopped | Booting | Running | Error`,
  `adb_serial?`, `grpc_port?`, `uptime?`, `pid?`

### EmuProfile  (the `.emuprofile` recipe)
- `schema_version`, `name`, `platform: "android"`
- `device` { profile: String }   (custom profiles inline their definition under `device.custom`)
- `image` { api, type, abi }
- `hardware` { ram_mb, storage_gb, dpi, ... }
- `seed?` { apks: [filename], settings: { locale, ... } }
- Round-trips with `SystemImage.coord` + `Emulator.hardware`.

### Plan / RequirementDiff  (result of resolving a profile)
- `diff`: list of `Requirement { kind: Emulator|PlatformTools|SystemImage|..., status: Present|NeedsDownload { size_bytes }, coord? }`
- `create_spec`: the `CreateSpec` to run once downloads finish.

### HostReport
- `os`, `arch`, `virtualization: Enabled | DisabledInFirmware | Unknown`
- `accelerator` { kind: Kvm|Whpx|Aehd|Hvf|None, status: Ok | Missing | NoPermission | ... }
- `disk_free_bytes`, `ram_bytes`
- `verdict: CanAccelerate | Degraded { reason } | CannotRun { reason }`
- `fixes[]`: `Fix { id, title, scriptable: bool, needs_reboot: bool, description }`

### Job
- `id`, `kind: Download|EnsureImage|Create|Launch|ApplyProfile|Update`, `state: Queued|Running|Done|Failed|Cancelled`
- `phase`, `pct`, `eta_secs`, `started_at`, `finished_at`, `error?`
- log lines streamed, not all stored (tail kept)

## Relationships

```
DeviceProfile 1───* Emulator *───1 SystemImage(coord)
EmuProfile ──resolve──> Plan(RequirementDiff + CreateSpec) ──apply──> Emulator
Emulator 1───* Job (history)
HostReport (singleton-ish, snapshotted on probe)
```

## Invariants

- An `Emulator` row always maps to exactly one on-disk AVD; `reconcile()` deletes rows whose AVD
  vanished and adopts AVDs created out-of-band (marked `source: Manual`, `discovered: true`).
- `SystemImage.installed` is derived from the filesystem at read time, never trusted from cache.
- A recipe with `platform != "android"` never produces a `Plan` — it's rejected at parse.
- `image.abi` must be launchable on `HostReport.arch` (x86_64 image on arm64 host → rejected with
  guidance to pick an arm64 image).
