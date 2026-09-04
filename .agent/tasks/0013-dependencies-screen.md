---
id: "0013"
title: "IPC + Dependencies screen wired to real toolchain state"
milestone: "M1"
status: "done"
owner: "Claude Code"
created: "2026-09-05"
updated: "2026-09-05"
---

## Goal

The Dependencies screen (currently static, from task 0003) shows the real component catalog,
real installed state, and live download/bootstrap progress, driven by new typed commands.

## Context / links

- Depends on: task 0010 (catalog), task 0012 (`InstalledState`/`bootstrap`)
- Architecture: `docs/architecture.md` §4 (IPC contract, `job://progress`/`job://log`/`job://done`
  event shapes)
- Design: `docs/design/wireframes/` screen 3

## Scope — files this task may touch

- `src-tauri/src/commands.rs` (or a new `src-tauri/src/commands/toolchain.rs`) — `list_components`,
  `installed_state`, `bootstrap_toolchain` commands
- `src-tauri/src/lib.rs` (register commands + event emitters)
- `src/lib/ipc.ts` (query hooks, following the `usePing` pattern)
- `src/routes/` — the Dependencies screen component + its test
- `just bindings` diff (generated `src/lib/bindings.ts`)

## Acceptance criteria

- [x] `list_components` returns the M1 catalog (task 0010) with `installed: bool` merged in from
      `InstalledState`
- [x] `bootstrap_toolchain` kicks off task 0012's `bootstrap()` as a background job, emitting
      `job://progress`/`job://log`/`job://done` (or the M0-era placeholder event shape if the
      orchestrator's real event channel isn't built yet — if so, say so here and scope a follow-up)
- [x] Dependencies screen: component list with installed/not-installed state, a working
      "Install" action wired to `bootstrap_toolchain`, live progress + log tail while running
- [x] Frontend tests (Vitest + Testing Library) cover the installed/not-installed rendering and
      the progress-updates-while-running behavior with a mocked bindings module
- [x] `just bindings` is clean (`git diff --exit-code src/lib/bindings.ts`)
- [x] `just check-fast` passes

## Validate

```
just bindings && git diff --exit-code src/lib/bindings.ts
pnpm test
just check-fast
```

## Notes / findings

### No real job-event orchestrator exists — one typed event stands in, deliberately scoped down

There is no persistent multi-job registry anywhere in the codebase yet (`emu_core::model::job::Job`
defines the *shape* of a job, task 0002, but nothing stores or tracks instances of one — that's
M3's "Registry & reliable tracking" milestone). Rather than build that registry early to satisfy
architecture.md §4's literal `job://progress`/`job://log`/`job://done` event trio, this task adds
**one** `tauri-specta` typed event, `BootstrapProgress` (wire name `job://bootstrap`), whose payload
is a tagged `BootstrapProgressKind` enum (`Progress`/`Log`/`Done` variants) — the same information
architecture.md's three events carry, collapsed into one channel because there is only ever one
toolchain-bootstrap job in flight at a time (a fixed `JOB_ID = "toolchain-bootstrap"` constant, not
a generated id). Revisit when M3's real job registry lands — the frontend's `BootstrapRunState`
reducer (`src/lib/ipc.ts`) already isolates the "turn events into UI state" logic from the command
layer, so swapping in a real per-job id later is a small, contained change.

### Real bug found and fixed: `zip::ZipFile` isn't `Send`

Wiring `commands::toolchain::bootstrap_toolchain` (a real `#[tauri::command]`, whose async body
*must* produce a `Send` future — tauri's executor requires it) into task 0012's `bootstrap()` was
the first time that code path was ever exercised in a context that actually enforces `Send`.
`emu-core`'s own `#[tokio::test]`s never needed it, so this went uncaught until now. `zip::ZipFile`
holds a `&mut dyn Read` with no `Send` bound, so holding one across an `.await` (as the original
`extract_cmdline_tools` loop did — reading the next entry, then `.await`ing an `Fs` call, entry
still in scope) makes the *whole* future non-`Send`, and the compiler error only surfaces at the
outermost `Send`-requiring boundary (the tauri command), pointing at `bootstrap.rs` deep inside
`emu-core`. Fixed by splitting `extract_cmdline_tools` in `crates/emu-core/src/toolchain/
bootstrap.rs` into two phases: a plain, non-`async` `read_cmdline_tools_zip` that reads the whole
archive into owned, `Send`-safe data (`Vec<ExtractedEntry>`) with no `.await` anywhere in it, run
via `tokio::task::spawn_blocking` (real ~140 MB deflate decompression shouldn't block the async
executor either, a second, independent reason for the split) — then a separate async loop that
only ever touches that owned data plus the `Fs` port. No `zip`-crate type survives past the
`spawn_blocking` call. This is task 0012 code but was only caught and fixed here, in task 0013 —
recorded in both task files' Notes rather than only one.

### `u64` archive sizes can't cross the IPC seam directly

`specta-typescript` (`=0.0.12`) refuses to export `u64`/`usize`/`i64`/etc. to a plain TS `number`
(silent precision loss above 2^53). `ComponentInfo.size_bytes` (the one place a catalog size
crosses IPC) is `u32` instead, narrowed with a saturating `u32::try_from(...).unwrap_or(u32::MAX)`
at the one construction site — every real M1 archive, and even `docs/spec.md`'s largest quoted
future system image (~3.5 GB), fits comfortably under `u32::MAX` (~4.29 GB).

### `#[tauri::command]`-annotated functions can't be `pub use` re-exported

`#[tauri::command]`/`#[specta::specta]` generate hidden sibling items (`__cmd__<name>`,
`__specta__fn__<name>`, …) next to each function they annotate; `collect_commands!` looks those up
at the function's *defining* module path. A `pub use toolchain::{list_components, ...}` re-export
in `commands/mod.rs` moves the visible name but not those siblings, so `lib.rs`'s
`collect_commands!` failed to find them. Fixed by keeping `mod toolchain` `pub(crate)` (not
re-exporting its commands) and referencing them as `commands::toolchain::list_components` /
`commands::toolchain::bootstrap_toolchain` directly in `lib.rs`. `BootstrapProgress` (a plain
struct with a derive, not an attribute-macro-annotated function) doesn't have this problem, but is
referenced the same way for consistency.

### `crate::ports`'s blanket `#[allow(dead_code, unused_imports)]` is gone

Task 0011 left a module-scoped allow on `src-tauri/src/ports/mod.rs`, explicitly flagged there to
be removed "the moment task 0012 calls any of these for real." Task 0012 didn't touch `src-tauri`
at all, so it fell to this task. Removing it surfaced two more pre-existing, real issues the
blanket allow had been silently hiding: a genuinely unused `AsyncWriteExt` import in
`ports/downloader.rs`'s own test module (fixed — `use super::*` already brought it in) and
`SystemClock` itself being unconstructed anywhere (still true — nothing in M1 needs the wall
clock through this seam; given its own narrow, honest `#[allow(dead_code)]` instead, since M2's
launch tracking is the expected first real caller).

### Not done here

- No real per-job registry / multi-job tracking (see above) — M3.
- No download pause/resume/cancel surfaced in the UI (task 0011 didn't build it into
  `Downloader::fetch` either — `MILESTONES.md`'s fuller M1 vision, still deferred).
- `sdkmanager --list` output parsing for real system images — not part of M1's three components.
