# E2E smoke suite

Drives the **built** Tauri app through WebdriverIO and
[`tauri-driver`](https://v2.tauri.app/develop/tests/webdriver/). Self-contained: its own
`package.json` / `node_modules`, **not** part of the root pnpm workspace, so it never affects
`just validate`.

## Prerequisites

- **Linux or Windows.** `tauri-driver` has no macOS support — on a Mac use
  [`docs/playbooks/macos-e2e-checklist.md`](../docs/playbooks/macos-e2e-checklist.md).
- `cargo install tauri-driver --locked`
- Linux: `sudo apt-get install -y webkit2gtk-driver` (provides `WebKitWebDriver`)

## Run

```sh
just e2e            # from the repo root: builds the debug app, then runs the suite
```

or directly:

```sh
pnpm tauri build --debug --no-bundle    # from the repo root
cd e2e && npm ci && npm test
```

## Layout

- `wdio.conf.ts` — spawns/kills `tauri-driver`, points `capabilities` at
  `src-tauri/target/debug/<binary>` (a small candidate list covers a future `mainBinaryName`).
- `specs/smoke.e2e.ts` — the one spec: app launches, Dashboard + sidebar render, the typed IPC
  seam reaches a `pong`. Creating + booting an emulator is a follow-up spec (see
  `.agent/tasks/0030`).

## CI

Runs in the `e2e` job of `.github/workflows/ci.yml` (ubuntu + windows) and the nightly. Both are
currently gated on the account's GitHub Actions billing block, same as the rest of CI.
