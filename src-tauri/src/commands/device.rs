//! Device inspector commands (task 0038): a live `adb logcat` stream and a quick device-facts
//! read for the emulator detail panel.
//!
//! `start_logcat` spawns `adb -s <serial> logcat -v threadtime` through the shared
//! [`AndroidProvider`] and drains it on a background task, emitting each line as a
//! `device://log` event. `stop_logcat` kills it. `device_facts` runs a handful of `adb shell`
//! one-shots and returns what parsed.

use emu_core::model::emulator::EmulatorId;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_specta::Event as _;

use crate::ipc_error::IpcError;
use crate::provider_state::ManagedProvider;

/// One line of a running emulator's `adb logcat`, emitted as `device://log`. The frontend filters
/// by `id` and parses the `-v threadtime` line itself (level / tag / message).
#[derive(Debug, Clone, Serialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "device://log")]
#[serde(rename_all = "camelCase")]
pub struct DeviceLogLine {
    /// The tracked emulator id this line belongs to.
    pub id: String,
    /// The raw logcat line, or a synthetic `— … —` marker when the stream ends.
    pub line: String,
}

/// A quick snapshot of a running emulator for the inspector's facts strip.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFactsDto {
    pub model: Option<String>,
    pub android_release: Option<String>,
    pub sdk_int: Option<u32>,
    pub battery_pct: Option<u8>,
    /// Free space on `/data`, MB.
    pub data_free_mb: Option<u32>,
    /// Total size of `/data`, MB.
    pub data_total_mb: Option<u32>,
}

/// Start streaming `adb logcat` for a running emulator. Lines arrive on `device://log`; the
/// stream stops on [`stop_logcat`], on app exit, or when the device goes away.
#[tauri::command]
#[specta::specta]
pub async fn start_logcat(
    app: AppHandle,
    mgr: State<'_, ManagedProvider>,
    id: String,
) -> Result<(), IpcError> {
    let provider = mgr.get().await?;
    let emu_id = EmulatorId(id.clone());
    let handle = provider
        .logcat_start(&emu_id)
        .await
        .map_err(IpcError::from)?;

    tokio::spawn(async move {
        loop {
            let next = {
                let mut child = handle.lock().await;
                child.next_line().await
            };
            let Ok(Some(line)) = next else {
                let _ = DeviceLogLine {
                    id: id.clone(),
                    line: "— logcat stream ended —".to_string(),
                }
                .emit(&app);
                break;
            };
            let _ = DeviceLogLine {
                id: id.clone(),
                line,
            }
            .emit(&app);
        }
    });

    Ok(())
}

/// Stop the `adb logcat` stream for `id`. Idempotent — a no-op if none is running.
#[tauri::command]
#[specta::specta]
pub async fn stop_logcat(mgr: State<'_, ManagedProvider>, id: String) -> Result<(), IpcError> {
    let provider = mgr.get().await?;
    provider.logcat_stop(&EmulatorId(id)).await;
    Ok(())
}

/// One network interface address for the inspector's Network panel.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NetInterfaceDto {
    pub name: String,
    /// e.g. `10.0.2.16/24`.
    pub addr: String,
}

/// One open IPv4 socket for the inspector's Network panel.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NetConnectionDto {
    /// `tcp` / `udp`.
    pub proto: String,
    pub local: String,
    pub remote: String,
    /// TCP state; empty for UDP.
    pub state: String,
    pub uid: u32,
    pub package: Option<String>,
}

/// Live network state of a running emulator.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeviceNetworkDto {
    pub interfaces: Vec<NetInterfaceDto>,
    pub connections: Vec<NetConnectionDto>,
}

/// Interface addresses + open sockets of a running emulator (socket-level, IPv4 — this is not an
/// HTTP inspector; that needs an in-app agent Emulator Studio doesn't inject).
#[tauri::command]
#[specta::specta]
pub async fn device_network(
    mgr: State<'_, ManagedProvider>,
    id: String,
) -> Result<DeviceNetworkDto, IpcError> {
    let provider = mgr.get().await?;
    let net = provider
        .device_network(&EmulatorId(id))
        .await
        .map_err(IpcError::from)?;
    Ok(DeviceNetworkDto {
        interfaces: net
            .interfaces
            .into_iter()
            .map(|i| NetInterfaceDto {
                name: i.name,
                addr: i.addr,
            })
            .collect(),
        connections: net
            .connections
            .into_iter()
            .map(|c| NetConnectionDto {
                proto: c.proto.to_string(),
                local: c.local,
                remote: c.remote,
                state: c.state.to_string(),
                uid: c.uid,
                package: c.package,
            })
            .collect(),
    })
}

/// Model / Android version / battery / `/data` usage for a running emulator.
#[tauri::command]
#[specta::specta]
pub async fn device_facts(
    mgr: State<'_, ManagedProvider>,
    id: String,
) -> Result<DeviceFactsDto, IpcError> {
    let provider = mgr.get().await?;
    let facts = provider
        .device_facts(&EmulatorId(id))
        .await
        .map_err(IpcError::from)?;
    let mb = |v: Option<u64>| v.map(|n| u32::try_from(n).unwrap_or(u32::MAX));
    Ok(DeviceFactsDto {
        model: facts.model,
        android_release: facts.android_release,
        sdk_int: facts.sdk_int,
        battery_pct: facts.battery_pct,
        data_free_mb: mb(facts.data_free_mb),
        data_total_mb: mb(facts.data_total_mb),
    })
}
