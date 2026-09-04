# Playbook: add or change an IPC command

The Rust↔TS seam is generated. Follow this exactly or CI fails on the bindings diff.

## Steps

1. **Put the logic in `emu-core`**, not in the command. The command is a thin adapter.
2. In `src-tauri/src/commands.rs` add the function:
   ```rust
   #[tauri::command]
   #[specta::specta]
   pub async fn list_images(state: State<'_, App>, filter: ImageFilter)
       -> Result<Vec<SystemImage>, IpcError> {
       state.core.list_images(filter).await.map_err(IpcError::from)
   }
   ```
3. Register it in the `tauri_specta::Builder` command list in `src-tauri/src/lib.rs`.
4. Every type used in the signature derives `serde::{Serialize,Deserialize}` + `specta::Type`.
   Put shared models in `emu-core`, not `src-tauri`.
5. Errors: return `IpcError`; its `code` comes from `CoreError::code()`. Add a new variant +
   `code` string if needed and cover the mapping in a Rust test.
6. **Regenerate bindings:**
   ```
   just bindings
   git diff src/lib/bindings.ts     # review it; it should reflect exactly your change
   ```
7. **Consume it** in `src/lib/ipc.ts` with a typed wrapper / TanStack Query hook. Never call
   `invoke` directly from a component.
8. **Tests:** Rust test for the error mapping / input validation; frontend test that mocks
   `src/lib/bindings.ts` and asserts the hook's behavior.
9. `just check-fast`, then `just validate`. The bindings check is
   `git diff --exit-code src/lib/bindings.ts` after a fresh `just bindings`.
10. Update `docs/architecture.md` §4 if you added an event or changed the error contract.

## Events

Emit from `src-tauri` with a typed payload struct (`specta::Type`); document the topic in
`docs/architecture.md` §4 (`job://progress`, `registry://changed`, …). The frontend subscribes in
`src/lib/events.ts`.

## Gotcha

`lefthook` pre-commit runs `just bindings` and stages the result, so a normal `git commit` keeps
you honest. If you commit with `--no-verify`, you own the diff.
