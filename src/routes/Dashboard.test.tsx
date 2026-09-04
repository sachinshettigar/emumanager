import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { Dashboard } from "./Dashboard";
import { commands } from "../lib/bindings";

vi.mock("../lib/bindings", () => ({
  commands: { ping: vi.fn() },
}));

const pingMock = vi.mocked(commands.ping);

function renderDashboard(): void {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <Dashboard />
    </QueryClientProvider>,
  );
}

describe("Dashboard backend status", () => {
  beforeEach(() => {
    pingMock.mockReset();
  });

  it("shows 'pong from vX.Y.Z' once the backend replies", async () => {
    pingMock.mockResolvedValue({
      status: "ok",
      data: { message: "pong, EmuManager", version: "1.2.3" },
    });

    renderDashboard();

    await waitFor(() => {
      expect(screen.getByTestId("backend-status")).toHaveTextContent(
        "pong, EmuManager from v1.2.3",
      );
    });
  });

  it("shows the backend error message when the command fails", async () => {
    pingMock.mockResolvedValue({
      status: "error",
      error: { code: "process_failed", message: "adb not found", details: null },
    });

    renderDashboard();

    await waitFor(() => {
      expect(screen.getByTestId("backend-status")).toHaveTextContent(
        "backend error: adb not found",
      );
    });
  });
});
