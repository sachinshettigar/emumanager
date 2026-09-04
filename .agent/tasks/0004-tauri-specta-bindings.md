---
id: "0004"
title: "tauri-specta wired: ping command + generated bindings"
milestone: "M0"
status: "todo"
owner: ""
created: "2026-09-04"
updated: "2026-09-04"
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

## Acceptance criteria

- [ ] `ping(name: String) -> Result<Pong, IpcError>` where `Pong { message: String, version: String }`
- [ ] `IpcError { code: String, message: String, details: Option<serde_json::Value> }`, `code`
      sourced from `CoreError::code()`
- [ ] `just bindings` regenerates `src/lib/bindings.ts`; running it twice is a no-op
      (`git diff --exit-code src/lib/bindings.ts`)
- [ ] `bindings.ts` is generated (has a header marking it generated) and imported by `src/lib/ipc.ts`
- [ ] A `usePing()` TanStack Query hook; Dashboard renders "pong from vX.Y.Z"
- [ ] Frontend test mocks `bindings.ts` and asserts the hook surfaces the message
- [ ] `cargo test -p app` (or wherever the command lives) covers the error mapping

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts && pnpm typecheck && pnpm test
```

## Notes / findings
