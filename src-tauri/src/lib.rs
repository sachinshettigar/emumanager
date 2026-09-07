//! Emulator Studio Tauri shell.
//!
//! This crate is **glue only** — Tauri commands, event emitters, and dependency
//! wiring. All domain logic lives in `emu-core` and the provider crates
//! (`AGENTS.md` §6, `docs/architecture.md` §2).
//!
//! The Rust↔TS seam is typed: [`specta_builder`] registers every command with
//! `tauri-specta`, which the `export_bindings` test renders into
//! `src/lib/bindings.ts` (`just bindings`). Never hand-edit that file.

#![deny(unsafe_code)]

mod commands;
mod ipc_error;
mod ports;
mod provider_state;

use tauri::Manager as _;
use tauri_specta::{collect_commands, collect_events, Builder};

use provider_state::ManagedProvider;

pub use commands::Pong;
pub use ipc_error::IpcError;

/// The single `tauri-specta` builder — the source of truth for the IPC surface.
///
/// `run()` mounts it on the real Tauri app; the `export_bindings` test renders
/// it to TypeScript. Both call this so the generated bindings can never drift
/// from what the app actually serves.
fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::ping,
            commands::app_info,
            commands::toolchain::list_components,
            commands::toolchain::bootstrap_toolchain,
            commands::toolchain::install_component,
            commands::toolchain::uninstall_component,
            commands::emulator::list_devices,
            commands::emulator::list_images,
            commands::emulator::list_emulators,
            commands::emulator::reconcile_now,
            commands::emulator::create_emulator,
            commands::emulator::launch_emulator,
            commands::emulator::stop_emulator,
            commands::emulator::rename_emulator,
            commands::emulator::edit_hardware,
            commands::emulator::delete_emulator,
            commands::emulator::wipe_emulator_data,
            commands::emulator::emulator_detail,
            commands::emulator::emulator_log_tail,
            commands::emulator::reveal_path,
            commands::profile::inspect_profile,
            commands::profile::apply_profile,
            commands::profile::export_profile,
            commands::profile::export_profile_to_file,
            commands::profile::save_profile,
            commands::profile::list_profiles,
            commands::profile::get_saved_profile,
            commands::profile::delete_profile,
            commands::host::probe_host,
            commands::host::run_helper,
            commands::device::start_logcat,
            commands::device::stop_logcat,
            commands::device::device_facts,
        ])
        .events(collect_events![
            commands::toolchain::BootstrapProgress,
            commands::emulator::EmulatorJob,
            commands::device::DeviceLogLine,
        ])
}

/// Build and run the desktop application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            // One shared AndroidProvider for the app's lifetime (see `provider_state`).
            let os = emu_core::model::HostOs::current();
            let data_dir = app.path().app_data_dir().ok();
            app.manage(ManagedProvider::new(data_dir, os));

            // Converge the registry to on-disk / adb reality once at startup — spawned, not
            // blocking, so a slow `avdmanager`/`adb` never holds up the window. Best-effort: a
            // failure (no SDK installed yet, adb missing) is recovered by the 4s Dashboard poll
            // and the next explicit action.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Some(mgr) = handle.try_state::<ManagedProvider>() {
                    if let Ok(provider) = mgr.get().await {
                        use emu_core::provider::Provider as _;
                        let _ = provider.reconcile().await;
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the Emulator Studio application")
        .run(|handle, event| {
            // On quit, reap every emulator child the provider still holds so we never orphan one
            // we launched. Time-boxed — a stuck child must not wedge shutdown.
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(provider) = handle
                    .try_state::<ManagedProvider>()
                    .and_then(|mgr| mgr.peek())
                {
                    tauri::async_runtime::block_on(async {
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_secs(8),
                            provider.shutdown(),
                        )
                        .await;
                    });
                }
            }
        });
}

#[cfg(test)]
mod export {
    use super::specta_builder;
    use specta_typescript::Typescript;

    /// Regenerate `src/lib/bindings.ts`. Run via `just bindings`; CI asserts the
    /// checked-in file matches (`git diff --exit-code src/lib/bindings.ts`).
    #[test]
    fn export_bindings() {
        let header = "\
// @generated by tauri-specta — run `just bindings` to regenerate.
/* eslint-disable */
// prettier-ignore
";
        specta_builder()
            .export(
                Typescript::default().header(header),
                "../src/lib/bindings.ts",
            )
            .expect("failed to export TypeScript bindings");
    }
}
