# macOS E2E checklist (manual)

`tauri-driver` has **no macOS support** (<https://v2.tauri.app/develop/tests/webdriver/>), so the
automated E2E suite (`just e2e`, task `0030`) only runs on Linux and Windows. On a Mac, run this
checklist by hand before a release — it is the same flow M2's Definition of Done describes.

Takes ~10 minutes on a machine that already has the SDK installed; the first run (cold SDK) can
take much longer because of the system-image download.

Each numbered list below restarts at 1; do the sections in order.

## Prep

1. `just dev` (or launch a release build).
2. The window opens on **My emulators** (the Dashboard). The sidebar shows Dashboard,
   Dependencies, Create, Profiles, About. The "Host" strip shows a readiness verdict and the
   backend-status line ends in `pong, Emulator Studio from v<version>`.

## SDK from zero (skip if already installed)

1. Go to **Dependencies**. If components are missing, click **Install all** (or install each
   from its row). The progress panel streams `sdkmanager` output; the bar is determinate during
   the `cmdline-tools` download and an indeterminate pulse during the `sdkmanager` phase.
2. When it finishes, every component row shows **installed (app-managed)**.
3. The **Host** panel shows a verdict. On Apple Silicon it should be
   *"Ready — hardware accelerated"* (HVF). If it says *degraded* / *cannot run*, note the reason.

## Create + launch a Play Store x86_64 emulator

1. Go to **Create**.
   - **Device:** expand **Phones**, pick **Pixel 6** (or similar). Leave "Show device frame" on.
   - **System image:** pick a **Play Store** `x86_64` image (download size shown; it will be
     fetched on create). On Apple Silicon an `arm64-v8a` image boots faster — either is fine for
     the checklist, note which you used.
   - **Hardware:** leave the defaults (2048 MB RAM, 6144 MB storage).
   - **Review → Create & launch.**
2. The job panel streams: image download (if needed) → `creating AVD …` → `launching…`.
3. Back on the **Dashboard**, the new row goes **stopped → booting… → running** (the "booting…"
   state shows a pulsing amber dot). An emulator window appears; if the device has a skin
   installed you see the bezel, otherwise just the screen (the job log says which).

## Inspect it

1. Open the emulator's detail page (click its name).
2. In the **Device** section: the facts strip shows a model, `Android <ver> · API <n>`, a
   battery %, and `/data` free/total. Click **Start logcat** — lines stream in. Set the level
   filter to **W+** and confirm Info lines drop out. Type a tag and confirm it narrows. Click
   **Pause** (the view freezes) then **Clear**.

## Stop + clean up

1. On the Dashboard row (or the detail page) click **Stop**. The row returns to **stopped**
   within ~15 s and the emulator window closes.
2. (Optional) **Export profile** from the row → a `.emuprofile` is written to
   `<data-dir>/exports/` and revealed in Finder.
3. (Optional) On **Dependencies**, click **Export diagnostics** → a
   `diagnostics-<timestamp>.zip` is written and revealed. Open it and confirm
   `emulator-studio.log` has no absolute `/Users/<you>/…` paths (they show as `~/…`).
4. Delete the test emulator (detail page → **Delete** → confirm, with "also remove the AVD"
   checked).

## Record the result

Note in the release PR / changelog: OS version, chip (M-series / Intel), which system image, and
any step that did not behave as described.
