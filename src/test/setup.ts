import "@testing-library/jest-dom/vitest";
import { afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

afterEach(() => {
  cleanup();
});

// There is no Tauri host under jsdom. Default every command invocation to a
// rejection so component tests exercise the "backend unavailable" path
// deterministically; tests needing a specific reply mock `../lib/bindings`.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.reject(new Error("Tauri unavailable in tests"))),
}));

// Same reasoning for the event API `bindings.ts`'s generated `events.*` helpers wrap
// (`@tauri-apps/api/event`): no real Tauri host to listen on under jsdom. `listen`/`once`
// resolve to a no-op unlisten function so a component's subscribe-on-mount effect doesn't throw;
// tests that need a specific event to fire mock `../lib/bindings`'s `events` export directly
// (same pattern as commands), not this module.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  once: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(() => Promise.resolve()),
}));

// jsdom has no matchMedia; provide a minimal stub so components/tests that
// probe prefers-color-scheme / prefers-reduced-motion don't throw.
if (!window.matchMedia) {
  window.matchMedia = (query: string): MediaQueryList =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }) as MediaQueryList;
}
