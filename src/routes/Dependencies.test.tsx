import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { Dependencies } from "./Dependencies";
import { commands, events } from "../lib/bindings";
import type { BootstrapProgress, ComponentInfo } from "../lib/bindings";

vi.mock("../lib/bindings", () => ({
  commands: {
    listComponents: vi.fn(),
    bootstrapToolchain: vi.fn(),
    installComponent: vi.fn(),
    uninstallComponent: vi.fn(),
    probeHost: vi.fn(),
    runHelper: vi.fn(),
    exportDiagnostics: vi.fn(),
    revealPath: vi.fn().mockResolvedValue({ status: "ok", data: null }),
  },
  events: {
    jobBootstrap: { listen: vi.fn() },
  },
}));

const listComponentsMock = vi.mocked(commands.listComponents);
const bootstrapToolchainMock = vi.mocked(commands.bootstrapToolchain);
const installComponentMock = vi.mocked(commands.installComponent);
const uninstallComponentMock = vi.mocked(commands.uninstallComponent);
const probeHostMock = vi.mocked(commands.probeHost);
const runHelperMock = vi.mocked(commands.runHelper);
const listenMock = vi.mocked(events.jobBootstrap.listen);

const HOST_OK = {
  status: "ok" as const,
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
};

const HOST_NO_KVM = {
  status: "ok" as const,
  data: {
    os: "linux",
    arch: "x86_64",
    virtualization: "enabled",
    acceleratorKind: "kvm",
    acceleratorStatus: "noPermission",
    diskFreeMb: 100_000,
    ramMb: 16_384,
    verdict: "degraded",
    verdictReason: "your user account can't use KVM yet",
    fixes: [
      {
        id: "add-kvm-group",
        title: "Add your user to the kvm group",
        scriptable: true,
        needsReboot: false,
        description: "Runs sudo usermod -aG kvm $USER.",
      },
    ],
  },
};

const CMDLINE_TOOLS: ComponentInfo = {
  id: "cmdline-tools;latest",
  name: "cmdline-tools",
  version: "19.0",
  sizeBytes: 150_000_000,
  installed: true,
  source: "app-managed",
};

const PLATFORM_TOOLS: ComponentInfo = {
  id: "platform-tools",
  name: "platform-tools (adb)",
  version: "36.0.0",
  sizeBytes: 12_000_000,
  installed: false,
  source: null,
};

const EMULATOR: ComponentInfo = {
  id: "emulator",
  name: "emulator",
  version: "35.1",
  sizeBytes: 900_000_000,
  installed: false,
  source: null,
};

const EMULATOR_INSTALLED: ComponentInfo = {
  ...EMULATOR,
  installed: true,
  source: "app-managed",
};

const PLATFORM_TOOLS_SYSTEM: ComponentInfo = {
  ...PLATFORM_TOOLS,
  installed: true,
  source: "system:/opt/android-sdk",
};

function renderDependencies(): void {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <Dependencies />
    </QueryClientProvider>,
  );
}

describe("Dependencies screen", () => {
  let emit: (payload: BootstrapProgress) => void = () => {
    throw new Error("emit called before jobBootstrap.listen resolved");
  };

  beforeEach(() => {
    listComponentsMock.mockReset();
    bootstrapToolchainMock.mockReset();
    installComponentMock.mockReset();
    uninstallComponentMock.mockReset();
    probeHostMock.mockReset();
    runHelperMock.mockReset();
    probeHostMock.mockResolvedValue(HOST_OK);
    listenMock.mockReset();
    listenMock.mockImplementation((cb) => {
      // The real bindings pass a full tauri `Event<T>`; only `.payload` is ever read.
      emit = (payload) => {
        cb({ payload } as Parameters<typeof cb>[0]);
      };
      return Promise.resolve(() => {});
    });
  });

  it("shows installed components with their source and an Install button for the rest", async () => {
    listComponentsMock.mockResolvedValue({
      status: "ok",
      data: [CMDLINE_TOOLS, PLATFORM_TOOLS],
    });

    renderDependencies();

    await waitFor(() => {
      expect(screen.getByTestId("component-cmdline-tools;latest")).toHaveTextContent(
        "installed (app-managed)",
      );
    });
    expect(screen.getByTestId("install-platform-tools")).toBeInTheDocument();
    expect(screen.queryByTestId("install-cmdline-tools;latest")).not.toBeInTheDocument();
  });

  it("installs a single component when its row button is clicked", async () => {
    listComponentsMock.mockResolvedValue({ status: "ok", data: [CMDLINE_TOOLS, PLATFORM_TOOLS] });
    installComponentMock.mockResolvedValue({ status: "ok", data: null });

    renderDependencies();
    await waitFor(() => {
      expect(screen.getByTestId("install-platform-tools")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("install-platform-tools"));

    await waitFor(() => {
      expect(installComponentMock).toHaveBeenCalledWith("platform-tools");
    });
  });

  it("uninstalls an app-managed component after a confirm step", async () => {
    listComponentsMock.mockResolvedValue({
      status: "ok",
      data: [CMDLINE_TOOLS, EMULATOR_INSTALLED],
    });
    uninstallComponentMock.mockResolvedValue({ status: "ok", data: null });

    renderDependencies();
    await waitFor(() => {
      expect(screen.getByTestId("uninstall-emulator")).toBeInTheDocument();
    });
    // The command-line tools can never be removed from the UI.
    expect(screen.queryByTestId("uninstall-cmdline-tools;latest")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("uninstall-emulator"));
    expect(uninstallComponentMock).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId("uninstall-confirm-emulator"));
    await waitFor(() => {
      expect(uninstallComponentMock).toHaveBeenCalledWith("emulator");
    });
  });

  it("offers no Uninstall for a component found in a system SDK", async () => {
    listComponentsMock.mockResolvedValue({
      status: "ok",
      data: [CMDLINE_TOOLS, PLATFORM_TOOLS_SYSTEM],
    });

    renderDependencies();
    await waitFor(() => {
      expect(screen.getByTestId("component-platform-tools")).toHaveTextContent(
        "installed (system:/opt/android-sdk)",
      );
    });
    expect(screen.queryByTestId("uninstall-platform-tools")).not.toBeInTheDocument();
  });

  it("hides the Install-all button once every component is installed", async () => {
    listComponentsMock.mockResolvedValue({ status: "ok", data: [CMDLINE_TOOLS] });

    renderDependencies();

    await waitFor(() => {
      expect(screen.getByTestId("component-cmdline-tools;latest")).toHaveTextContent("installed");
    });
    expect(screen.queryByTestId("install-button")).not.toBeInTheDocument();
    expect(screen.queryByTestId("install-cmdline-tools;latest")).not.toBeInTheDocument();
  });

  it("surfaces a catalog load failure", async () => {
    listComponentsMock.mockResolvedValue({
      status: "error",
      error: { code: "download_failed", message: "could not reach dl.google.com", details: null },
    });

    renderDependencies();

    await waitFor(() => {
      expect(screen.getByText(/could not reach dl.google.com/)).toBeInTheDocument();
    });
  });

  it("streams live progress and log lines while installing, then shows completion", async () => {
    listComponentsMock.mockResolvedValue({ status: "ok", data: [PLATFORM_TOOLS, EMULATOR] });
    bootstrapToolchainMock.mockResolvedValue({ status: "ok", data: null });

    renderDependencies();
    await waitFor(() => {
      expect(screen.getByTestId("install-button")).not.toBeDisabled();
    });

    fireEvent.click(screen.getByTestId("install-button"));
    await waitFor(() => {
      expect(bootstrapToolchainMock).toHaveBeenCalledOnce();
    });

    act(() => {
      emit({
        jobId: "toolchain-bootstrap",
        payload: { type: "log", line: "downloading platform-tools" },
      });
      emit({
        jobId: "toolchain-bootstrap",
        payload: { type: "progress", phase: "Installing platform-tools", pct: 42 },
      });
    });

    await waitFor(() => {
      expect(screen.getByTestId("bootstrap-log")).toHaveTextContent("downloading platform-tools");
    });
    expect(screen.getByText("Installing platform-tools")).toBeInTheDocument();
    expect(screen.getByText("42%")).toBeInTheDocument();
    expect(screen.getByTestId("install-button")).toHaveTextContent("Installing…");

    const bar = screen.getByTestId("bootstrap-bar");
    expect(bar).toHaveAttribute("data-indeterminate", "false");
    expect(bar.firstElementChild).toHaveStyle({ width: "42%" });

    // A phase with no percentage (the sdkmanager download step) → indeterminate bar.
    act(() => {
      emit({
        jobId: "toolchain-bootstrap",
        payload: { type: "progress", phase: "Downloading & installing emulator", pct: null },
      });
    });
    await waitFor(() => {
      expect(screen.getByTestId("bootstrap-bar")).toHaveAttribute("data-indeterminate", "true");
    });

    act(() => {
      emit({ jobId: "toolchain-bootstrap", payload: { type: "done", ok: true, error: null } });
    });

    await waitFor(() => {
      expect(screen.getByText("Install complete")).toBeInTheDocument();
    });
  });

  it("shows the real error when a run fails", async () => {
    listComponentsMock.mockResolvedValue({ status: "ok", data: [PLATFORM_TOOLS, EMULATOR] });
    bootstrapToolchainMock.mockResolvedValue({
      status: "error",
      error: { code: "process_failed", message: "sdkmanager exited with 1", details: null },
    });

    renderDependencies();
    await waitFor(() => {
      expect(screen.getByTestId("install-button")).not.toBeDisabled();
    });

    fireEvent.click(screen.getByTestId("install-button"));

    await waitFor(() => {
      expect(screen.getByText("sdkmanager exited with 1")).toBeInTheDocument();
    });
  });

  it("renders the host verdict and tiles", async () => {
    listComponentsMock.mockResolvedValue({ status: "ok", data: [CMDLINE_TOOLS] });
    renderDependencies();

    await waitFor(() => {
      expect(screen.getByTestId("host-verdict")).toHaveAttribute("data-verdict", "canAccelerate");
    });
    expect(screen.getByTestId("host-panel")).toHaveTextContent("hvf · ok");
  });

  it("shows a scriptable fix and runs it", async () => {
    listComponentsMock.mockResolvedValue({ status: "ok", data: [CMDLINE_TOOLS] });
    probeHostMock.mockResolvedValue(HOST_NO_KVM);
    runHelperMock.mockResolvedValue({
      status: "ok",
      data: {
        command: "add-kvm-group",
        status: "ok",
        message: "Added you to the kvm group. Log out and back in.",
        needsReboot: false,
      },
    });
    renderDependencies();

    await waitFor(() => {
      expect(screen.getByTestId("fix-add-kvm-group")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("fix-run-add-kvm-group"));

    await waitFor(() => {
      expect(runHelperMock).toHaveBeenCalledWith("add-kvm-group");
      expect(screen.getByTestId("fix-result")).toHaveTextContent("kvm group");
    });
  });

  it("exports a diagnostics bundle and reveals it", async () => {
    listComponentsMock.mockResolvedValue({ status: "ok", data: [CMDLINE_TOOLS] });
    vi.mocked(commands.exportDiagnostics).mockResolvedValue({
      status: "ok",
      data: "/data/diagnostics-1750000000.zip",
    });
    renderDependencies();

    await waitFor(() => {
      expect(screen.getByTestId("export-diagnostics")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("export-diagnostics"));

    await waitFor(() => {
      expect(vi.mocked(commands.exportDiagnostics)).toHaveBeenCalled();
      expect(screen.getByTestId("diagnostics-path")).toHaveTextContent(
        "diagnostics-1750000000.zip",
      );
      expect(vi.mocked(commands.revealPath)).toHaveBeenCalledWith(
        "/data/diagnostics-1750000000.zip",
      );
    });
  });
});
