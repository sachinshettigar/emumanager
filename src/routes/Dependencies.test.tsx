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
    probeHost: vi.fn(),
    runHelper: vi.fn(),
  },
  events: {
    jobBootstrap: { listen: vi.fn() },
  },
}));

const listComponentsMock = vi.mocked(commands.listComponents);
const bootstrapToolchainMock = vi.mocked(commands.bootstrapToolchain);
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

  it("shows installed and not-installed components with their source", async () => {
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
    expect(screen.getByTestId("component-platform-tools")).toHaveTextContent("not installed");
  });

  it("disables Install once every component is already installed", async () => {
    listComponentsMock.mockResolvedValue({ status: "ok", data: [CMDLINE_TOOLS] });

    renderDependencies();

    await waitFor(() => {
      expect(screen.getByTestId("install-button")).toHaveTextContent("All installed");
    });
    expect(screen.getByTestId("install-button")).toBeDisabled();
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
    listComponentsMock.mockResolvedValue({ status: "ok", data: [PLATFORM_TOOLS] });
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

    act(() => {
      emit({ jobId: "toolchain-bootstrap", payload: { type: "done", ok: true, error: null } });
    });

    await waitFor(() => {
      expect(screen.getByText("Install complete")).toBeInTheDocument();
    });
  });

  it("shows the real error when a run fails", async () => {
    listComponentsMock.mockResolvedValue({ status: "ok", data: [PLATFORM_TOOLS] });
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
});
