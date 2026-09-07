---
id: "0031"
title: "First-run onboarding + per-component SDK install + About screen"
milestone: "M6"
status: "review"
owner: "Claude Code"
created: "2026-09-07"
updated: "2026-09-07"
---

## Goal

Make the empty first run legible: a short onboarding checklist on the Dashboard, the ability to
install each SDK component on its own (not just "install everything"), and an About screen with the
version, license and a one-line-per-feature summary.

## Context / links

- User request (session 9): "small onboarding steps for first time; allow each SDK to be installed
  separately; add license & version info with a short description of all features."
- `MILESTONES.md` M6 (first-run hardening); `docs/spec.md` §3 (features), §7 (zero-terminal setup)
- `crates/emu-core/src/toolchain::bootstrap` already takes an arbitrary `wanted: &[Component]` —
  a single-component install is just a one-element slice.
- `src-tauri/src/commands/toolchain.rs` (`bootstrap_toolchain`, `list_components`, `resolve_catalog`,
  `scan_installed`, the `job://bootstrap` event); `src/routes/Dependencies.tsx`, `Dashboard.tsx`,
  `src/nav.ts`, `src/router.tsx`

## Scope — files this task may touch

- `src-tauri/src/commands/toolchain.rs` — `install_component(component_id)` (reuses `bootstrap`)
- `src-tauri/src/commands/mod.rs` — `app_info() -> AppInfo { name, version, license, repository, description, features }`
- `src-tauri/src/lib.rs` — register both
- `src/lib/bindings.ts` (regenerated), `src/lib/ipc.ts` — `useInstallComponent`, `useAppInfo`
- `src/routes/Dependencies.tsx` — per-row "Install"; bulk button → "Install all missing"
- `src/routes/Dashboard.tsx` — an `OnboardingChecklist` card (auto-hides when setup is complete)
- `src/routes/About.tsx` (new) + `src/routes/About.test.tsx` (new); `src/nav.ts`, `src/router.tsx`,
  `src/routes/routes.test.tsx`
- `src/routes/Dependencies.test.tsx` / `Dashboard.test.tsx` — mock the new commands
- `PROGRESS.md`, `.agent/state.json`, `MILESTONES.md`, journal

## Acceptance criteria

- [x] `install_component(componentId)` (`commands/toolchain.rs`) — filters `resolve_catalog` to the
      one component, then the shared `run_bootstrap` helper (`toolchain::bootstrap` for a
      one-element `wanted`) on the same `job://bootstrap` channel; `not_found` for an unknown id;
      an already-installed component is `bootstrap`'s clean no-op. `bootstrap_toolchain` was
      refactored onto the same `run_bootstrap`.
- [x] Dependencies: each not-installed `ComponentRow` has its own "Install" button
      (`useInstallComponent`); the top button is now "Install all (N)" and shows **only when >1**
      component is missing (one missing → just use its row button). Live progress panel shared.
- [x] `app_info() -> AppInfo { name, version, license, repository, description, features }` —
      `version` = `CARGO_PKG_VERSION`, `license` = `CARGO_PKG_LICENSE` (`"UNLICENSED"` today,
      shown as "UNLICENSED — all rights reserved (OSS license TBD)"), `features` = 9 one-line
      entries. A plain `#[tauri::command]` (no `Result`, no I/O).
- [x] `/about` route + "About" nav item (+ an `about` `NavIcon`): name / version / license /
      repo link / the feature list. `routes.test.tsx` `/about` case added.
- [x] Dashboard `OnboardingChecklist`: renders while any of {an SDK component not installed, host
      `verdict === "cannotRun"`, zero emulators} holds; 3 numbered steps with ✓ when done and a
      link to the resolving screen; returns `null` once all three are satisfied. The stale "Host
      detection lands in M5" strip was also replaced with a live one-line `HostSummary`.
- [x] Vitest: Dependencies (per-row install calls `install_component`; "Install all" hidden when
      everything's installed); `About.test.tsx` (version + license + feature + repo link);
      `Dashboard.test.tsx` (onboarding card shows with no emulators, gone once one exists). 38 web
      tests.
- [x] `just bindings` clean once committed; `just check-fast` then `just validate` green (154 rust
      tests). `knip` green — every new hook has a consumer.

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
just validate
```

## Notes / findings

### `bootstrap` already took an arbitrary `wanted`

`emu_core::toolchain::bootstrap(data_dir, wanted: &[Component], …)` was already generic over the
component list — `bootstrap_toolchain` just happened to pass the whole catalog. `install_component`
passes a one-element slice; both now share a `run_bootstrap` helper in `commands/toolchain.rs`
(ports + `JobHandle` + the terminal `job://bootstrap` event). No `emu-core` change.

### License

`option_env!("CARGO_PKG_LICENSE")` resolves to `"UNLICENSED"` (the workspace `license` field, which
`src-tauri` inherits). The About screen shows it as `UNLICENSED — all rights reserved (OSS license
TBD)` — honest about the current state; picking a real licence is M7's `LICENSE`-chosen DoD line.
`option_env!` (not `env!`) so a future build without the field still compiles.

### "Install all" visibility

Shown only when **more than one** component is missing — with a single gap the per-row button is
the obvious action and a redundant top button is noise. Hidden entirely when nothing's missing
(was previously a disabled "All installed" button). The existing tests that clicked `install-button`
now use a 2-missing fixture (`[PLATFORM_TOOLS, EMULATOR]`).

### Onboarding "SDK ready"

= `components.data` loaded **and** every row `installed`. Host "ready" = a report loaded and
`verdict !== "cannotRun"` (a `degraded` host still counts as ready for onboarding — you can create
and launch). "First emulator" = `list_emulators` non-empty. All three true → the card unmounts.

### Test-mock churn

`OnboardingChecklist` calls `useComponents` + `useHostReport` on the Dashboard, so
`Dashboard.test.tsx`'s bindings mock gained `listComponents` (+ it already had `probeHost` from
`0027`). `About.test.tsx` is new with its own `appInfo` mock. `routes.test.tsx`'s `/about` case
relies on the global `@tauri-apps/api/core` reject — the `<Screen title="About">` heading renders
even when `app_info` "fails", so the heading assertion passes.
