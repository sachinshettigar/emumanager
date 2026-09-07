import { describe, it, expect } from "vitest";
import { screen } from "@testing-library/react";
import { renderRoute } from "../test/renderRoute";

describe("route rendering", () => {
  const cases: readonly { path: string; heading: RegExp }[] = [
    { path: "/", heading: /my emulators/i },
    { path: "/create", heading: /create emulator/i },
    { path: "/dependencies", heading: /dependencies & sdk/i },
    { path: "/profiles", heading: /^profiles$/i },
    { path: "/about", heading: /^about$/i },
  ];

  for (const { path, heading } of cases) {
    it(`renders ${path} without error`, () => {
      renderRoute(path);
      expect(screen.getByRole("heading", { level: 1, name: heading })).toBeInTheDocument();
      // The persistent shell is present on every route.
      expect(screen.getByRole("navigation", { name: /primary/i })).toBeInTheDocument();
    });
  }
});
