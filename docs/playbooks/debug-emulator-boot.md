# Playbook: an emulator won't create or boot

Work top to bottom. Capture anything surprising into a test fixture and cite it.

## 1. Reproduce outside the app

Using the app's managed SDK dir (`<data_dir>/sdk`), set `ANDROID_SDK_ROOT` and
`ANDROID_AVD_HOME` to the app's paths, then:

```
$SDK/cmdline-tools/latest/bin/sdkmanager --list_installed
$SDK/cmdline-tools/latest/bin/avdmanager list avd
$SDK/emulator/emulator -list-avds
$SDK/emulator/emulator @<name> -verbose -no-snapshot -no-window   # watch the log
$SDK/platform-tools/adb devices
```

If it fails here, it's an SDK/host problem, not an app bug.

## 2. Accelerator

```
$SDK/emulator/emulator -accel-check
```

- Linux: `ls -l /dev/kvm`, `groups | grep kvm`. Fix → `emu-helper add-kvm-group` (elevated).
- Windows: WHPX feature enabled? `emu-helper check`. HAXM is EOL — never rely on it.
- macOS: HVF is built-in on Apple Silicon; on Intel check virtualization isn't disabled.
- No nested virt (CI, some VMs) → expect `CannotRun`; that's correct behavior, test it.

## 3. Image / ABI mismatch

- x86_64 image on an arm64 host (or vice versa) → won't boot. `emu-host` should reject this at
  create time with guidance. Verify `HostReport.arch` vs `image.abi`.

## 4. Boot-complete detection

The app waits for:

```
adb -s <serial> shell getprop sys.boot_completed    # "1"
adb -s <serial> shell getprop init.svc.bootanim     # "stopped"
```

If the emulator process is up but the app still says "Booting", the poll/parse in
`emu-android` is the suspect — add a fixture from a real `getprop` dump and test the parser.

## 5. Cold boot vs. snapshot

A corrupt snapshot hangs boot. Try `-no-snapshot-load` (the app's "Cold boot" option). If cold
boot works and warm doesn't, the snapshot handling is the bug.

## 6. Graphics

Headless CI: force `-gpu swiftshader_indirect`. Wayland: window may not appear — see the spec
open question; prefer `-no-window` + screenshot via `adb exec-out screencap` for tests.

## 7. Still stuck

Capture `emulator -verbose` output + `HostReport` into the task's `## Notes`, set the task
`blocked`, and write a journal entry with the exact command that fails.
