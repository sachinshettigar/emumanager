import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { About } from "./About";
import { commands } from "../lib/bindings";

vi.mock("../lib/bindings", () => ({
  commands: { appInfo: vi.fn() },
  events: {},
}));

const appInfoMock = vi.mocked(commands.appInfo);

function renderAbout(): void {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <About />
    </QueryClientProvider>,
  );
}

describe("About", () => {
  beforeEach(() => {
    appInfoMock.mockReset();
  });

  it("shows the version, license and feature list", async () => {
    appInfoMock.mockResolvedValue({
      name: "Emulator Studio",
      version: "0.1.0",
      license: "MIT",
      repository: "https://github.com/sachinshettigar/emumanager",
      description: "Create, launch, track and share Android emulators.",
      features: [
        "Installs the Android SDK into its own directory.",
        "Portable .emuprofile recipes.",
      ],
    });
    renderAbout();

    await waitFor(() => {
      expect(screen.getByTestId("about-version")).toHaveTextContent("0.1.0");
    });
    expect(screen.getByTestId("about-license")).toHaveTextContent("MIT");
    expect(screen.getByTestId("about-features")).toHaveTextContent("Portable .emuprofile recipes");
    expect(screen.getByRole("link", { name: /github.com\/sachinshettigar/ })).toBeInTheDocument();
  });
});
