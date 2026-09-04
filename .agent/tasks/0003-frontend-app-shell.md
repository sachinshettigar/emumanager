---
id: "0003"
title: "Vite + React + TS strict app shell with 4 routes"
milestone: "M0"
status: "done"
owner: "Claude Code"
created: "2026-09-04"
updated: "2026-09-05"
---

## Goal

The frontend renders the app shell from the wireframes: sidebar nav + four routes, static
content, dark/light aware, strict TS, tests + lint wired.

## Scope — files this task may touch

- `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, `vite.config.ts`, `index.html`
- `eslint.config.js`, `.prettierrc`, `vitest.config.ts`
- `src/main.tsx`, `src/App.tsx`, `src/routes/{Dashboard,Create,Dependencies,Profiles}.tsx`
- `src/components/AppShell.tsx`, `src/components/Sidebar.tsx`
- `src/styles/tokens.css`, `tailwind.config.ts`, `postcss.config.js`
- `src/**/*.test.tsx`

## Acceptance criteria

- [x] `tsconfig.app.json`: `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`
      (+ `noImplicitOverride`, `noUnused*`); project-references layout (`tsconfig.json` →
      `tsconfig.app.json` / `tsconfig.node.json`)
- [~] ESLint flat config with `typescript-eslint` strict-type-checked + stylistic-type-checked +
      `react-hooks` + `react-refresh`; `pnpm lint` clean. **`eslint-plugin-import` deferred** —
      flat-config + resolver friction, no value for the M0 surface; tracked as a follow-up.
- [x] Prettier configured (`.prettierrc.json`, scoped `.prettierignore`); `pnpm format:check` clean
- [x] Router (`createBrowserRouter`) with `/`, `/create`, `/dependencies`, `/profiles`; `Sidebar`
      uses `NavLink` so the active route gets `aria-current="page"`; `/` is exact-match only
- [x] `tokens.css` defines the palette (IBM Plex Sans/Mono, blue `#2f6db3`, green `#2f8a5f`,
      amber `#b07d2b`, neutrals) as `--em-*` CSS vars surfaced as Tailwind semantic utilities;
      components use `bg-surface` / `text-muted` / … — no raw hex in components
- [x] `prefers-color-scheme` (light + `[data-theme]` overrides both ways) and
      `prefers-reduced-motion` handled in `tokens.css`
- [x] Vitest + Testing Library: `routes.test.tsx` renders all 4 routes; `Sidebar.test.tsx`
      asserts active-state + exact-match; `pnpm test` green (7 tests); v8 coverage reporter on
- [x] `pnpm build` (tsc -b + vite) succeeds

## Validate

```
pnpm install && pnpm typecheck && pnpm lint && pnpm test && pnpm build
```

## Notes / findings

- **Stack pinned:** React 18.3.1, react-router-dom 6.28 (`createBrowserRouter`), Vite 5.4,
  Tailwind 3.4 (v4 deferred — big config-model change), Vitest 2.1 + Testing Library 16,
  typescript-eslint 8.15 flat config, TypeScript 5.6.
- **ESLint flat config shape:** typed rules (`strictTypeChecked` + `stylisticTypeChecked`) are
  scoped to `src/**/*.{ts,tsx}` via `projectService`. Root config files and `scripts/**` get a
  `disableTypeChecked` block with `parser: tseslint.parser` + `globals.node` (otherwise `.ts`
  config files hit "Unexpected token {" and `.mjs` scripts hit `no-undef` on `console`/`process`).
- **Nav single-sourced:** `src/nav.ts` `NAV_ITEMS` feeds both `router.tsx` and `Sidebar.tsx`.
- **`aria-label="Primary"` lives on `<nav>`, not `<aside>`** — `aside` has role `complementary`;
  the test queries `role="navigation"`.
- **React Router v7 future flags** all enabled (`routerFuture` in `router.tsx`, shared with the
  test memory-router) to keep console output clean.
- **`dist/` quirk carried over from 0001:** committed placeholder `dist/index.html` is what
  `cargo build` / `generate_context!` embeds on a bare clone. `pnpm build` (also
  `beforeBuildCommand` in `tauri.conf.json`) overwrites `dist/` with the real Vite output —
  `.gitignore` keeps `dist/*` except `!dist/index.html`, so after a local `pnpm build` run
  `git checkout dist/index.html` (or ignore the diff — CI always rebuilds). Proper fix = M6.
- Fonts: IBM Plex loaded from Google Fonts via `<link>` in `index.html` (CSP is `null`).
  Bundling for offline = M7.
- `@vitejs/plugin-react` + esbuild: pnpm 10 ignores esbuild's build script by default but Vite
  vendors its own esbuild, so no `onlyBuiltDependencies` entry is needed — verified that
  `pnpm test` and `pnpm build` both work.
