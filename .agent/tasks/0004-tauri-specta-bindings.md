---
id: "0004"
title: "tauri-specta wired: ping command + generated bindings"
milestone: "M0"
status: "done"
owner: "Claude Code"
created: "2026-09-04"
updated: "2026-09-05"
---

## Goal

The typed IPC seam exists: a Rust `ping` command, `src/lib/bindings.ts` generated from it by
`tauri-specta`, and the frontend calls it through a TanStack Query hook.

## Scope — files this task may touch

- `src-tauri/Cargo.toml`, `src-tauri/src/{lib.rs,commands.rs,ipc_error.rs}`
- `src-tauri/build.rs` (specta export in debug/test)
- `src/lib/bindings.ts` (generated — do not hand-edit)
- `src/lib/ipc.ts` (typed wrappers / query hooks)
- `src/routes/Dashboard.tsx` (show ping result)
- `justfile` (`bindings` recipe)
- also touched: `package.json` (add `@tanstack/react-query`, `@tauri-apps/api`),
  `src/main.tsx` (QueryClientProvider), `src/test/{setup.ts,renderRoute.tsx}`
  (mock `@tauri-apps/api/core`, wrap in a QueryClient), `eslint.config.js` +
  `.prettierignore` + `vitest.config.ts` (exclude the generated file),
  `clippy.toml` (`TypeScript`/`JavaScript`/`JSON` doc idents), and the two new
  frontend test files.

## Acceptance criteria

- [x] `ping(name: String) -> Result<Pong, IpcError>` where `Pong { message: String, version: String }`
- [x] `IpcError { code: String, message: String, details: Option<serde_json::Value> }`, `code`
      sourced from `CoreError::code()` (via `impl From<CoreError> for IpcError`)
- [x] `just bindings` regenerates `src/lib/bindings.ts`; running it twice is a no-op
      (`git diff --exit-code src/lib/bindings.ts`) — recipe runs the `export::export_bindings`
      test in crate `emumanager`
- [x] `bindings.ts` is generated (tauri-specta stamps "generated … Do not edit" + our
      `@generated` / eslint-disable header) and imported by `src/lib/ipc.ts`
- [x] A `usePing()` TanStack Query hook; Dashboard renders "… from v0.1.0" (full string
      "pong, EmuManager from v0.1.0")
- [x] Frontend tests: `src/lib/ipc.test.tsx` mocks `./bindings` and asserts the hook surfaces
      `message`/`version` and, on an error envelope, an `IpcCallError` carrying `.ipc`;
      `src/routes/Dashboard.test.tsx` asserts the rendered "from vX.Y.Z" line
- [x] `cargo test -p emumanager` covers the error mapping (`ipc_error::tests`,
      `commands::tests`) — 6 tests

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts && pnpm typecheck && pnpm test
```

Also run: `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo fmt --all --check`, `pnpm lint`, `pnpm format:check`,
`bash scripts/emu-core-no-tauri.sh` — all green.

## Notes / findings

### Versions — pinned together (bump in one commit)

| crate | version | why exact |
| --- | --- | --- |
| `tauri-specta` | `=2.0.0-rc.25` | v2 is a release candidate; the upstream docs insist on `=` |
| `specta` | `=2.0.0-rc.25` | `tauri-specta` pins it with `=`; features `derive`, `serde_json` (ours) + `function` (theirs) |
| `specta-typescript` | `=0.0.12` | must match the `specta` it was built against |
| `@tauri-apps/api` | `2.11.1` | JS side of `invoke`, matches `tauri` 2.11.x |
| `@tanstack/react-query` | `5.102.8` | command/event data layer (AGENTS.md §2) |

### Decisions / deviations

- **`specta::Type` on `emu-core` DTOs is still deferred** — the 0002 note said 0004 would do
  the crate-wide `#[derive(specta::Type)]` pass. The hard part (choosing + pinning the
  `specta`/`tauri-specta`/`specta-typescript` triple) is done here, but **no `emu-core` type
  crosses the IPC seam yet** (only `ping`), and 0004's file scope is `src-tauri` + frontend.
  The mechanical derive pass moves to the first M1 task that returns an `emu-core` type from a
  command, where it can be verified end to end. `emu-core` stays `tauri`-free; when the pass
  lands it adds `specta` (not `tauri-specta`) there, behind no feature (specta is cheap and
  `no_std`-friendly).
- **`IpcError.details` is exported as TS `unknown`**, not `JsonValue`. `serde_json::Value`
  inlines `serde_json::Number`, which specta refuses to export (it contains `i64`/`u64` and
  specta forbids BigInt-precision-loss types). `#[specta(type = specta_typescript::Unknown)]`
  on the field sidesteps it; `unknown` is the honest TS type for arbitrary JSON anyway.
- **`ping` takes `name: String`** (not `&str`) because `#[tauri::command]` deserializes args
  from the IPC payload. `commands.rs` carries a module-level
  `#![allow(clippy::needless_pass_by_value)]` with that rationale.
- **Frontend error type is `IpcCallError extends Error`**, wrapping the backend `IpcError` on
  `.ipc`. Throwing the bare serialized object trips `@typescript-eslint/only-throw-error`, and
  a real `Error` behaves better with error boundaries / stack traces. `usePing()` is typed
  `UseQueryResult<Pong, IpcCallError>`.
- **The generated file is excluded from eslint / prettier / coverage** (it has its own
  "do not edit" header) so `just validate` doesn't fight the generator over formatting.
- **`src-tauri/build.rs` untouched** — the export runs in a `#[cfg(test)]` module, not the
  build script, so a normal `cargo build` doesn't need to write into `src/`.

### Known wart (not this task)

`pnpm test` prints a React Router v7 `startTransition` future-flag warning from
`Sidebar.test.tsx` / `routes.test.tsx`. Tests pass; the app passes all 6 future flags in
`src/router.tsx`. Chase it when the router setup is next touched (or on the v7 bump).
