---
id: "0003"
title: "Vite + React + TS strict app shell with 4 routes"
milestone: "M0"
status: "todo"
owner: ""
created: "2026-09-04"
updated: "2026-09-04"
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

- [ ] `tsconfig.json`: `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`
- [ ] ESLint flat config with `typescript-eslint` strict-type-checked + `react-hooks` + `import`;
      `pnpm lint` clean
- [ ] Prettier configured; `pnpm format:check` clean
- [ ] Router with `/`, `/create`, `/dependencies`, `/profiles`; sidebar highlights active
- [ ] `tokens.css` defines the wireframe palette (IBM Plex Sans/Mono, blue primary `#2f6db3`,
      green `#2f8a5f`, amber `#b07d2b`, neutrals) as CSS vars; components consume vars, not hex
- [ ] Respects `prefers-color-scheme` and `prefers-reduced-motion`
- [ ] Vitest + Testing Library: a test per route renders without error; a Sidebar test asserts
      active-state; `pnpm test` green with coverage reporter on
- [ ] `pnpm build` (vite) succeeds

## Validate

```
pnpm install && pnpm typecheck && pnpm lint && pnpm test && pnpm build
```

## Notes / findings
