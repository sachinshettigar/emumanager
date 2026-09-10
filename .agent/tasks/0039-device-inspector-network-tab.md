---
id: "0039"
title: "Device inspector v2 — Logcat/Network tabs + a socket-level network panel"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-09"
updated: "2026-09-09"
---

## Goal

User asked "can I see logs? or network?" and "build replica UI for those". The logcat viewer
(task 0038) already exists; this gives it more room and adds a **Network** tab — a socket-level
view (interfaces + open TCP/UDP sockets) that's the honest, agent-free approximation of Android
Studio's Network Inspector.

## Context / links

- Android Studio's Network Inspector intercepts OkHttp/HttpURLConnection via an `androidx.inspection`
  agent injected into a debuggable app — not reachable from `adb`. This is **not** that; it reads
  `/proc/net` and `ip addr` over `adb shell`.
- `/proc/net/tcp` format: kernel `Documentation/networking/proc_net_tcp.rst` — space-separated
  rows, `local`/`rem` are little-endian hex `IIIIIIII:PPPP`, col 3 is the hex TCP state, col 7 is
  the uid.
- `ip -o addr`: one address per line, `<idx>: <iface> <family> <addr>/<prefix> …`.
- `pm list packages -U`: `package:<name> uid:<n>` per line.
- `crates/emu-android/src/provider.rs` — `device_facts` is the sibling; `running_serial` +
  `adb_path` helpers already exist. `src-tauri/src/commands/device.rs` — the DTO/command layer.

## Scope — files this task touched

- `crates/emu-android/src/provider.rs` — `DeviceNetwork` / `NetInterface` / `NetConnection`
  structs; `AndroidProvider::device_network`; free parsers `parse_ip_addr`,
  `parse_pm_list_packages_u`, `decode_hex_addr`, `tcp_state`, `parse_proc_net` (+ 3 tests)
- `src-tauri/src/commands/device.rs` — the `device_network` command plus `DeviceNetworkDto`,
  `NetInterfaceDto`, `NetConnectionDto`
- `src-tauri/src/lib.rs` — register it. `src/lib/bindings.ts` regen.
- `src/lib/ipc.ts` — `useDeviceNetwork(id, enabled)` (4 s poll)
- `src/routes/EmulatorDetail.tsx` — `DeviceInspector` split into a facts strip + `LogcatTab`
  (console now `max-h-[28rem]`, `whitespace-pre-wrap`) + `NetworkTab` (interfaces list + a
  sortable/filterable connections table), tab switcher
- `src/routes/EmulatorDetail.test.tsx` — mock + a Network-tab test
- `clippy.toml` — `IPv4`/`IPv6`/`TCP`/`UDP` added to `doc-valid-idents`
- `PROGRESS.md`, `.agent/state.json`, journal

## Acceptance criteria

- [x] `device_network(id) -> { interfaces, connections }`. `connections` are IPv4 TCP+UDP sockets
      from `/proc/net/{tcp,udp}` with the little-endian hex address decoded to `a.b.c.d:port`, the
      TCP state name, the owning uid, and the package (from `pm list packages -U`) when resolvable.
      IPv6 rows are skipped (documented). All parsers defensive — a row that doesn't parse is
      dropped, never guessed.
- [x] 3 Rust parser tests (`parse_ip_addr`, `decode_hex_addr` incl. the v6 → `None` case,
      `parse_proc_net` incl. uid→package).
- [x] Detail panel: a **Logcat** / **Network** tab switch under the facts strip. Logcat tab is
      the existing viewer with a taller console. Network tab: interface addresses + a table
      (App/uid · Proto · State · Local · Remote), text filter, established-then-listener sort, a
      one-line "this is socket-level, not an HTTP inspector" note. Both tabs say "start the
      emulator" when it's not running.
- [x] Vitest: switching to the Network tab shows the mocked interface + connection.
- [x] `just validate` green (bar the expected `bindings.ts` diff step until commit).

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

- Extends the M6 device inspector (task 0038) — kept on M6 rather than opening M7.
- **Not HTTP inspection.** Deliberately. The panel's own copy says so. A real request inspector
  needs the inspection agent — out of scope and arguably out of reach for a no-IDE tool.
- `/proc/net/tcp6` parsing (IPv6) is the obvious follow-up; the v6 hex encoding (4 LE 32-bit
  words) is finicky and the emulator's traffic is ~always IPv4.
- `doc_markdown` flags `IPv4`/`IPv6`/`TCP`/`UDP` in doc comments — added to `clippy.toml`'s
  allow-list rather than backticking every occurrence (same call the repo already made for `SDK`,
  `ABI`, `adb`, …).
