import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router-dom";

import { EmulatorDetail } from "./EmulatorDetail";
import { save } from "@tauri-apps/plugin-dialog";
import { commands, events } from "../lib/bindings";
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
    exportProfileToPath: vi.fn(),
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
    startLogcat: vi.fn().mockResolvedValue({ status: "ok", data: null }),
    stopLogcat: vi.fn().mockResolvedValue({ status: "ok", data: null }),
    deviceFacts: vi.fn(),
    deviceNetwork: vi.fn(),
  },
  events: {
    jobEmulator: { listen: vi.fn().mockResolvedValue(() => {}) },
    deviceLog: { listen: vi.fn().mockResolvedValue(() => {}) },
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
    vi.mocked(save).mockReset();
    vi.mocked(commands.exportProfileToPath).mockReset();
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

  it("streams logcat and filters it by level and text", async () => {
    detailMock.mockResolvedValue({ status: "ok", data: { ...DETAIL, state: "running" } });
    vi.mocked(commands.deviceFacts).mockResolvedValue({
      status: "ok",
      data: {
        model: "sdk_gphone64_arm64",
        androidRelease: "14",
        sdkInt: 34,
        batteryPct: 100,
        dataFreeMb: 5000,
        dataTotalMb: 6000,
      },
    });
    let emitLog: (payload: { id: string; line: string }) => void = () => {
      throw new Error("deviceLog.listen not resolved");
    };
    vi.mocked(events.deviceLog.listen).mockImplementation((cb) => {
      emitLog = (payload) => {
        cb({ payload } as Parameters<typeof cb>[0]);
      };
      return Promise.resolve(() => {});
    });

    renderDetail();

    await waitFor(() => {
      expect(screen.getByTestId("device-facts")).toHaveTextContent("sdk_gphone64_arm64");
    });
    fireEvent.click(screen.getByTestId("logcat-toggle"));
    await waitFor(() => {
      expect(vi.mocked(commands.startLogcat)).toHaveBeenCalledWith("01J0ABC");
    });

    act(() => {
      emitLog({
        id: "01J0ABC",
        line: "01-02 03:04:05.678  1234  1234 I ActivityManager: hello world",
      });
      emitLog({
        id: "01J0ABC",
        line: "01-02 03:04:06.100  1234  1300 E AndroidRuntime: FATAL EXCEPTION",
      });
      emitLog({ id: "other-emulator", line: "01-02 03:04:07.000 1 1 I X: not mine" });
    });

    await waitFor(() => {
      expect(screen.getByTestId("logcat-console")).toHaveTextContent("hello world");
    });
    expect(screen.getByTestId("logcat-console")).not.toHaveTextContent("not mine");

    // Raise the minimum level to E — the Info line drops out.
    fireEvent.change(screen.getByTestId("logcat-level"), { target: { value: "E" } });
    await waitFor(() => {
      expect(screen.getByTestId("logcat-console")).not.toHaveTextContent("hello world");
    });
    expect(screen.getByTestId("logcat-console")).toHaveTextContent("FATAL EXCEPTION");
  });

  it("shows the network panel — interfaces and open sockets — on the Network tab", async () => {
    detailMock.mockResolvedValue({ status: "ok", data: { ...DETAIL, state: "running" } });
    vi.mocked(commands.deviceNetwork).mockResolvedValue({
      status: "ok",
      data: {
        interfaces: [{ name: "wlan0", addr: "10.0.2.16/24" }],
        connections: [
          {
            proto: "tcp",
            local: "10.0.2.16:55600",
            remote: "8.8.8.8:443",
            state: "ESTABLISHED",
            uid: 10123,
            package: "com.example.app",
          },
        ],
      },
    });
    renderDetail();

    await waitFor(() => {
      expect(screen.getByTestId("device-tab-network")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("device-tab-network"));

    await waitFor(() => {
      expect(screen.getByTestId("net-interfaces")).toHaveTextContent("10.0.2.16/24");
      expect(screen.getByTestId("net-connections")).toHaveTextContent("com.example.app");
      expect(screen.getByTestId("net-connections")).toHaveTextContent("8.8.8.8:443");
    });
  });

  it("picks a location then exports the profile there", async () => {
    detailMock.mockResolvedValue({ status: "ok", data: DETAIL });
    vi.mocked(save).mockResolvedValue("/Users/u/Desktop/pixel6_api34.emuprofile");
    vi.mocked(commands.exportProfileToPath).mockResolvedValue({ status: "ok", data: null });
    renderDetail();

    await waitFor(() => {
      expect(screen.getByTestId("export-profile-file")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("export-profile-file"));

    await waitFor(() => {
      expect(vi.mocked(save)).toHaveBeenCalledWith(
        expect.objectContaining({ defaultPath: "pixel6_api34.emuprofile" }),
      );
      expect(vi.mocked(commands.exportProfileToPath)).toHaveBeenCalledWith(
        "01J0ABC",
        "/Users/u/Desktop/pixel6_api34.emuprofile",
      );
      expect(screen.getByTestId("exported-path")).toHaveTextContent("pixel6_api34.emuprofile");
    });
  });

  it("does nothing when the Save dialog is cancelled", async () => {
    detailMock.mockResolvedValue({ status: "ok", data: DETAIL });
    vi.mocked(save).mockResolvedValue(null);
    renderDetail();

    await waitFor(() => {
      expect(screen.getByTestId("export-profile-file")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("export-profile-file"));

    await waitFor(() => {
      expect(vi.mocked(save)).toHaveBeenCalled();
    });
    expect(vi.mocked(commands.exportProfileToPath)).not.toHaveBeenCalled();
    expect(screen.queryByTestId("exported-path")).not.toBeInTheDocument();
  });
});
