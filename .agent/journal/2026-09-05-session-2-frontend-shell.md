# 2026-09-05 — session 2 (Claude Code) — frontend app shell

## Worked on

- Task(s): `0003` (Vite + React + TS strict app shell with 4 routes)
- Milestone: `M0`

## Changed

- **Build/config:** `package.json` (real deps + scripts), `pnpm-lock.yaml`, `tsconfig.json` +
  `tsconfig.app.json` (strict, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`) +
  `tsconfig.node.json`, `vite.config.ts` (port 1420, `outDir: dist`), `vitest.config.ts`
  (jsdom, v8 coverage, `mergeConfig` over vite), `index.html` (root, IBM Plex `<link>`),
  `postcss.config.js`, `tailwind.config.ts` (semantic colors → `var(--em-*)`),
  `eslint.config.js` (flat, typed rules scoped to `src/**`), `.prettierrc.json`,
  `.prettierignore` (scoped to frontend), `.gitignore` (+`*.tsbuildinfo`).
- **Source:** `src/main.tsx`, `src/router.tsx` (+ `routerFuture` v7 flags), `src/App.tsx`,
  `src/nav.ts` (`NAV_ITEMS` — single source for router + sidebar), `src/index.css`
  (`@import tokens.css` + tailwind layers + base), `src/styles/tokens.css` (the palette),
  `src/components/{AppShell,Sidebar,Screen,icons}.tsx`,
  `src/routes/{Dashboard,Create,Dependencies,Profiles}.tsx` (static placeholders).
- **Tests:** `src/test/setup.ts` (jest-dom, cleanup, matchMedia stub),
  `src/test/renderRoute.tsx` (memory-router helper), `src/routes/routes.test.tsx` (4 routes),
  `src/components/Sidebar.test.tsx` (active-state + exact-match, 3 tests).
- **Tauri:** `src-tauri/tauri.conf.json` — added `devUrl`, `beforeDevCommand`,
  `beforeBuildCommand`.
- Progress files + this journal + `MILESTONES.md` (frontend box ticked).

## State now

- `pnpm typecheck` ✅ · `pnpm lint` ✅ (`--max-warnings 0`) · `pnpm format:check` ✅ ·
  `pnpm test` ✅ (7) · `pnpm build` ✅ · `just check-fast` ✅ (rust + web) ·
  `cargo build --workspace` ✅
- `just validate`: still not fully green — pre-0007. Missing local CLIs (cargo-deny, actionlint,
  gitleaks, markdownlint-cli2) and `cargo sqlx prepare --check` (needs task 0005).
- Tasks moved: `0003` todo → doing → done.
- `lastValidatedCommit` in state.json: null (still waiting on 0007's real gate).

## Next action

Task `0002` — `emu-core` ports and models (no impls). Per the task file: model structs
(`DeviceProfile`, `SystemImage`, `Emulator`, `EmuProfile`, `HostReport`, `Job`, …) with
`serde`; `ImageCoord` / `ImageType` / `Abi` with `Display` + `FromStr`; the port traits
(`ProcessRunner`, `Downloader`, `HostProbe`, `Clock`, `Fs`, `Provider`); a `testing` cargo
feature exposing fakes. Keep `emu-core` `tauri`-free (`scripts/emu-core-no-tauri.sh`).
`cargo test -p emu-core` is the gate.

## Gotchas / notes for the next agent

- `dist/index.html` is still the committed placeholder; `pnpm build` overwrites `dist/`.
  After a local build, `git checkout dist/index.html` before committing (assets are gitignored,
  so a committed built index.html would reference missing hashed files). CI rebuilds anyway.
  Real fix = M6 packaging.
- ESLint flat config: any NEW top-level `.ts`/`.mjs` tooling file must fall under the
  `*.{js,ts,mjs,cjs}` or `scripts/**` block (they set `parser: tseslint.parser` +
  `disableTypeChecked` + node globals). TS source stays under `src/**`.
- `eslint-plugin-import` still to be added (deferred). If you add it, prefer
  `eslint-plugin-import-x` for flat-config sanity.
- Vite dev server port **1420** is referenced in three places: `vite.config.ts`,
  `tauri.conf.json` `devUrl`. Keep them in sync.
- Adding a route = add to `src/nav.ts` `NAV_ITEMS` (router + sidebar both read it) and add an
  icon case in `src/components/icons.tsx` (`NavItem["icon"]` union is exhaustive-checked).
- `pnpm` 10.0.0 dropped the `pnpm` package.json field; if you need `onlyBuiltDependencies`
  later it goes in `pnpm-workspace.yaml`. esbuild works without it (Vite vendors its own).
