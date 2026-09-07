import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router-dom";

import { EmulatorDetail } from "./EmulatorDetail";
import { commands } from "../lib/bindings";
import type { EmulatorDetail as EmulatorDetailDto } from "../lib/bindings";

vi.mock("../lib/bindings", () => ({
  commands: {
    emulatorDetail: vi.fn(),
    emulatorLogTail: vi.fn(),
    renameEmulator: vi.fn(),
    editHardware: vi.fn(),
    deleteEmulator: vi.fn(),
    wipeEmulatorData: vi.fn(),
    launchEmulator: vi.fn(),
    stopEmulator: vi.fn(),
    revealPath: vi.fn(),
    exportProfile: vi.fn(),
    exportProfileToFile: vi.fn(),
    probeHost: vi.fn().mockResolvedValue({
      status: "ok",
      data: {
        os: "macos",
        arch: "aarch64",
        virtualization: "enabled",
        acceleratorKind: "hvf",
        acceleratorStatus: "ok",
        diskFreeMb: 400_000,
        ramMb: 32_768,
        verdict: "canAccelerate",
        verdictReason: "",
        fixes: [],
      },
    }),
    runHelper: vi.fn(),
  },
  events: {
    jobEmulator: { listen: vi.fn().mockResolvedValue(() => {}) },
  },
}));

const detailMock = vi.mocked(commands.emulatorDetail);
const logTailMock = vi.mocked(commands.emulatorLogTail);
const renameMock = vi.mocked(commands.renameEmulator);

const DETAIL: EmulatorDetailDto = {
  id: "01J0ABC",
  avdName: "pixel6_api34",
  displayName: "Pixel 6 · API 34",
  deviceProfileId: "pixel_6",
  imageCoord: "system-images;android-34;google_apis_playstore;x86_64",
  api: 34,
  hasPlayStore: true,
  ramMb: 2048,
  storageMb: 6144,
  graphics: "auto",
  deviceFrame: true,
  source: "created here",
  state: "stopped",
  adbSerial: null,
  createdAt: "2026-09-06T10:00:00Z",
  updatedAt: "2026-09-06T10:05:00Z",
  avdPath: "/home/u/.android/avd/pixel6_api34.avd",
};

function renderDetail(): void {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={["/emulator/01J0ABC"]}>
        <Routes>
          <Route path="/emulator/:id" element={<EmulatorDetail />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("EmulatorDetail", () => {
  beforeEach(() => {
    detailMock.mockReset();
    logTailMock.mockReset();
    renameMock.mockReset();
    logTailMock.mockResolvedValue({ status: "ok", data: [] });
  });

  it("renders the emulator's config and live state", async () => {
    detailMock.mockResolvedValue({ status: "ok", data: DETAIL });
    renderDetail();

    await waitFor(() => {
      expect(screen.getByTestId("detail-state")).toHaveTextContent("stopped");
    });
    expect(screen.getByText("pixel6_api34")).toBeInTheDocument();
    expect(screen.getByText(/API 34 · Play Store/)).toBeInTheDocument();
    expect(screen.getByTestId("ram-input")).toHaveValue(2048);
    expect(screen.getByTestId("open-folder")).toBeInTheDocument();
  });

  it("renames the emulator", async () => {
    detailMock.mockResolvedValue({ status: "ok", data: DETAIL });
    renameMock.mockResolvedValue({ status: "ok", data: null });
    renderDetail();

    await waitFor(() => {
      expect(screen.getByTestId("name-input")).toHaveValue("Pixel 6 · API 34");
    });
    fireEvent.change(screen.getByTestId("name-input"), { target: { value: "My phone" } });
    fireEvent.click(screen.getByTestId("rename-button"));

    await waitFor(() => {
      expect(renameMock).toHaveBeenCalledWith("01J0ABC", "My phone");
    });
  });

  it("shows the launch-log tail in the console", async () => {
    detailMock.mockResolvedValue({ status: "ok", data: DETAIL });
    logTailMock.mockResolvedValue({
      status: "ok",
      data: ["launching emulator @pixel6_api34", "emulator: boot completed"],
    });
    renderDetail();

    await waitFor(() => {
      expect(screen.getByTestId("log-console")).toHaveTextContent("boot completed");
    });
  });

  it("surfaces a load failure", async () => {
    detailMock.mockResolvedValue({
      status: "error",
      error: { code: "not_found", message: "emulator not found: 01J0ABC", details: null },
    });
    renderDetail();

    await waitFor(() => {
      expect(screen.getByText(/emulator not found/)).toBeInTheDocument();
    });
  });

  it("saves the profile to a file and shows the path", async () => {
    detailMock.mockResolvedValue({ status: "ok", data: DETAIL });
    vi.mocked(commands.exportProfileToFile).mockResolvedValue({
      status: "ok",
      data: "/home/u/.local/share/emulator-studio/exports/pixel6_api34.emuprofile",
    });
    renderDetail();

    await waitFor(() => {
      expect(screen.getByTestId("export-profile-file")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("export-profile-file"));

    await waitFor(() => {
      expect(vi.mocked(commands.exportProfileToFile)).toHaveBeenCalledWith("01J0ABC");
      expect(screen.getByTestId("exported-path")).toHaveTextContent("pixel6_api34.emuprofile");
    });
  });
});
