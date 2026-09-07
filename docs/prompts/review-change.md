Review this change (`<diff / PR link / branch>`) against Emulator Studio's standards.

Check, in order:
1. **Correctness** — does it do what the task's acceptance criteria say? Edge cases, error paths,
   concurrency (jobs, reconcile), resource cleanup (child processes, file handles).
2. **Boundaries** — `emu-core` has no `tauri` dep; domain logic isn't leaking into `src-tauri`;
   IPC only via generated bindings; SDK output parsed against fixtures with a cited source.
3. **Tests** — behavior-level, present for each criterion, no network / real Android binaries in
   the `just validate` path; a bug fix has a regression test.
4. **Security / safety** — no secrets or personal data; least-privilege Tauri capabilities;
   `emu-helper` invoked only on explicit user action; no shell string interpolation of untrusted
   input.
5. **Standards** — `AGENTS.md` §7 (Rust: `thiserror` in libs, `tracing` not `println!`, clippy
   clean; TS: strict, no `any`); user-facing strings are real and specific.
6. **Housekeeping** — `just bindings` / `just db-prepare` ran if relevant; progress files updated;
   ADR written for any real decision; docs updated if an interface changed.

Output: blocking issues first (with file:line), then non-blocking suggestions, then a one-line
verdict (approve / approve-with-nits / changes-needed). Don't rewrite the change — point at what
to fix.
