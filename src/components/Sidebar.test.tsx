import { describe, it, expect } from "vitest";
import { screen } from "@testing-library/react";
import { renderRoute } from "../test/renderRoute";
import { NAV_ITEMS } from "../nav";

describe("Sidebar", () => {
  it("lists every nav item", () => {
    renderRoute("/");
    for (const item of NAV_ITEMS) {
      expect(screen.getByRole("link", { name: item.label })).toBeInTheDocument();
    }
  });

  it("marks the active route with aria-current", () => {
    renderRoute("/dependencies");
    const active = screen.getByRole("link", { name: "Dependencies" });
    expect(active).toHaveAttribute("aria-current", "page");

    const inactive = screen.getByRole("link", { name: "Dashboard" });
    expect(inactive).not.toHaveAttribute("aria-current");
  });

  it("keeps the Dashboard link exact-match only (not active on nested routes)", () => {
    renderRoute("/create");
    const dashboard = screen.getByRole("link", { name: "Dashboard" });
    expect(dashboard).not.toHaveAttribute("aria-current");
  });
});
