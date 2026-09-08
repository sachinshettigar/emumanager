import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";

import { Dashboard } from "./Dashboard";
import { commands } from "../lib/bindings";
import { save } from "@tauri-apps/plugin-dialog";
import type { EmulatorInfo } from "../lib/bindings";

vi.mock("../lib/bindings", () => ({
  commands: {
    ping: vi.fn().mockResolvedValue({
      status: "ok",
      data: { message: "pong, Emulator Studio", version: "1.2.3" },
    }),
    listEmulators: vi.fn(),
    launchEmulator: vi.fn(),
    stopEmulator: vi.fn(),
    reconcileNow: vi.fn(),
    exportProfileToPath: vi.fn(),
    listComponents: vi.fn().mockResolvedValue({
      status: "ok",
      data: [
        {
          id: "emulator",
          name: "emulator",
          version: "35",
          sizeBytes: 1,
          installed: true,
          source: "app-managed",
        },
      ],
    }),
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
    jobBootstrap: { listen: vi.fn().mockResolvedValue(() => {}) },
    jobEmulator: { listen: vi.fn().mockResolvedValue(() => {}) },
  },
}));

const listEmulatorsMock = vi.mocked(commands.listEmulators);
const stopEmulatorMock = vi.mocked(commands.stopEmulator);
const reconcileNowMock = vi.mocked(commands.reconcileNow);
const probeHostMock = vi.mocked(commands.probeHost);

const RUNNING: EmulatorInfo = {
  id: "01J0RUNNING",
  avdName: "pixel6_api34",
  displayName: "Pixel 6 · API 34",
  state: "running",
  adbSerial: "emulator-5554",
};

const STOPPED: EmulatorInfo = {
  id: "01J0STOPPED",
  avdName: "tv_api33",
  displayName: "Android TV",
  state: "stopped",
  adbSerial: null,
};

function renderDashboard(): void {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("Dashboard", () => {
  beforeEach(() => {
    listEmulatorsMock.mockReset();
    stopEmulatorMock.mockReset();
    reconcileNowMock.mockReset();
  });

  it("shows the empty state when there are no emulators", async () => {
    listEmulatorsMock.mockResolvedValue({ status: "ok", data: [] });
    renderDashboard();
    await waitFor(() => {
      expect(screen.getByText(/No emulators yet/)).toBeInTheDocument();
    });
  });

  it("shows the onboarding checklist until setup is complete, then hides it", async () => {
    listEmulatorsMock.mockResolvedValue({ status: "ok", data: [] });
    renderDashboard();
    await waitFor(() => {
      expect(screen.getByTestId("onboarding")).toBeInTheDocument();
    });
    // "Create your first emulator" is the only outstanding step (SDK + host are ok in the mock).
    expect(screen.getByTestId("onboarding-step-2")).toHaveAttribute("href", "/create");

    listEmulatorsMock.mockResolvedValue({ status: "ok", data: [RUNNING] });
    // A fresh render with an emulator present: all three steps done → card gone.
    cleanup();
    renderDashboard();
    await waitFor(() => {
      expect(screen.getByTestId("emulator-01J0RUNNING")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("onboarding")).not.toBeInTheDocument();
  });

  it("lists emulators with their live state and the right action per row", async () => {
    listEmulatorsMock.mockResolvedValue({ status: "ok", data: [RUNNING, STOPPED] });
    renderDashboard();

    await waitFor(() => {
      expect(screen.getByTestId("emulator-state-01J0RUNNING")).toHaveTextContent("running");
    });
    expect(screen.getByTestId("emulator-01J0RUNNING")).toHaveTextContent("emulator-5554");
    expect(screen.getByTestId("stop-01J0RUNNING")).toBeInTheDocument();
    expect(screen.getByTestId("launch-01J0STOPPED")).toBeInTheDocument();
  });

  it("exports an emulator profile to a chosen location from its row", async () => {
    listEmulatorsMock.mockResolvedValue({ status: "ok", data: [STOPPED] });
    vi.mocked(save).mockResolvedValue("/Users/u/Desktop/tv_api33.emuprofile");
    vi.mocked(commands.exportProfileToPath).mockResolvedValue({ status: "ok", data: null });
    renderDashboard();

    await waitFor(() => {
      expect(screen.getByTestId("export-01J0STOPPED")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("export-01J0STOPPED"));

    await waitFor(() => {
      expect(vi.mocked(save)).toHaveBeenCalledWith(
        expect.objectContaining({ defaultPath: "tv_api33.emuprofile" }),
      );
      expect(vi.mocked(commands.exportProfileToPath)).toHaveBeenCalledWith(
        "01J0STOPPED",
        "/Users/u/Desktop/tv_api33.emuprofile",
      );
      expect(screen.getByTestId("export-01J0STOPPED")).toHaveTextContent("Exported");
    });
  });

  it("calls stop_emulator when Stop is clicked", async () => {
    listEmulatorsMock.mockResolvedValue({ status: "ok", data: [RUNNING] });
    stopEmulatorMock.mockResolvedValue({ status: "ok", data: null });
    renderDashboard();

    await waitFor(() => {
      expect(screen.getByTestId("stop-01J0RUNNING")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("stop-01J0RUNNING"));

    await waitFor(() => {
      expect(stopEmulatorMock).toHaveBeenCalledWith("01J0RUNNING");
    });
  });

  it("reconciles and repopulates the list when Refresh is clicked", async () => {
    listEmulatorsMock.mockResolvedValue({ status: "ok", data: [] });
    reconcileNowMock.mockResolvedValue({ status: "ok", data: [STOPPED] });
    renderDashboard();

    await waitFor(() => {
      expect(screen.getByText(/No emulators yet/)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("reconcile-button"));

    await waitFor(() => {
      expect(reconcileNowMock).toHaveBeenCalled();
      expect(screen.getByTestId("emulator-01J0STOPPED")).toHaveTextContent("Android TV");
    });
  });

  it("disables Launch and shows the reason when the host can't run emulators", async () => {
    listEmulatorsMock.mockResolvedValue({ status: "ok", data: [STOPPED] });
    probeHostMock.mockResolvedValueOnce({
      status: "ok",
      data: {
        os: "linux",
        arch: "x86_64",
        virtualization: "disabledInFirmware",
        acceleratorKind: "kvm",
        acceleratorStatus: "missing",
        diskFreeMb: 100_000,
        ramMb: 16_384,
        verdict: "cannotRun",
        verdictReason: "hardware virtualization is turned off in your firmware (BIOS/UEFI)",
        fixes: [],
      },
    });
    renderDashboard();

    await waitFor(() => {
      expect(screen.getByTestId("launch-01J0STOPPED")).toBeDisabled();
    });
    expect(screen.getByTestId("launch-blocked-01J0STOPPED")).toHaveTextContent("firmware");
  });

  it("surfaces a load failure", async () => {
    listEmulatorsMock.mockResolvedValue({
      status: "error",
      error: { code: "db_error", message: "registry locked", details: null },
    });
    renderDashboard();
    await waitFor(() => {
      expect(screen.getByText(/registry locked/)).toBeInTheDocument();
    });
  });

  it("still shows the backend liveness line", async () => {
    listEmulatorsMock.mockResolvedValue({ status: "ok", data: [] });
    renderDashboard();
    await waitFor(() => {
      expect(screen.getByTestId("backend-status")).toHaveTextContent(
        "pong, Emulator Studio from v1.2.3",
      );
    });
  });
});
