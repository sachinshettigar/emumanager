import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";

import { Profiles } from "./Profiles";
import { commands } from "../lib/bindings";
import type { ProfileInspection, ProfileSummary } from "../lib/bindings";

vi.mock("../lib/bindings", () => ({
  commands: {
    inspectProfile: vi.fn(),
    applyProfile: vi.fn(),
    listProfiles: vi.fn(),
    saveProfile: vi.fn(),
    deleteProfile: vi.fn(),
    getSavedProfile: vi.fn(),
    exportProfile: vi.fn(),
  },
  events: {
    jobEmulator: { listen: vi.fn().mockResolvedValue(() => {}) },
  },
}));

const inspectMock = vi.mocked(commands.inspectProfile);
const applyMock = vi.mocked(commands.applyProfile);
const listMock = vi.mocked(commands.listProfiles);
const deleteMock = vi.mocked(commands.deleteProfile);

const INSPECTION: ProfileInspection = {
  name: "QA baseline",
  description: "checkout regressions",
  deviceLabel: "pixel_6",
  imageLabel: "Android API 33 · google_apis_playstore · x86_64",
  imageCoord: "system-images;android-33;google_apis_playstore;x86_64",
  requirements: [
    { label: "Emulator", present: true, downloadBytes: 0 },
    { label: "System image · …", present: false, downloadBytes: 1_500_000_000 },
  ],
  totalDownloadBytes: 1_500_000_000,
  ready: false,
};

function renderProfiles(): void {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <Profiles />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

function dropFile(contents: string): void {
  const file = new File([contents], "x.emuprofile", { type: "application/json" });
  const input = screen.getByTestId("file-input");
  fireEvent.change(input, { target: { files: [file] } });
}

describe("Profiles", () => {
  beforeEach(() => {
    inspectMock.mockReset();
    applyMock.mockReset();
    listMock.mockReset();
    deleteMock.mockReset();
    listMock.mockResolvedValue({ status: "ok", data: [] });
  });

  it("shows the requirement preview for a dropped profile", async () => {
    inspectMock.mockResolvedValue({ status: "ok", data: INSPECTION });
    renderProfiles();

    dropFile('{"schemaVersion":"1.0"}');

    await waitFor(() => {
      expect(screen.getByTestId("preview")).toHaveTextContent("QA baseline");
    });
    expect(screen.getByTestId("requirements")).toHaveTextContent("download 1.4 GB");
    expect(screen.getByText(/Total download: 1.4 GB/)).toBeInTheDocument();
  });

  it("surfaces a specific rejection message", async () => {
    inspectMock.mockResolvedValue({
      status: "error",
      error: {
        code: "unsupported",
        message: 'this profile targets "ios", not Android',
        details: null,
      },
    });
    renderProfiles();

    dropFile('{"schemaVersion":"1.0","platform":"ios"}');

    await waitFor(() => {
      expect(screen.getByTestId("rejection")).toHaveTextContent("not Android");
    });
    expect(screen.queryByTestId("preview")).not.toBeInTheDocument();
  });

  it("applies a loaded profile", async () => {
    inspectMock.mockResolvedValue({ status: "ok", data: INSPECTION });
    applyMock.mockResolvedValue({ status: "ok", data: "01J0NEW" });
    renderProfiles();

    dropFile('{"schemaVersion":"1.0"}');
    await waitFor(() => {
      expect(screen.getByTestId("apply-button")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("apply-button"));

    await waitFor(() => {
      expect(applyMock).toHaveBeenCalledWith(expect.any(Array), false);
    });
  });

  it("lists saved profiles and deletes one", async () => {
    const saved: ProfileSummary = {
      name: "QA baseline",
      description: "checkout",
      createdAt: "2026-09-06T10:00:00Z",
    };
    listMock.mockResolvedValue({ status: "ok", data: [saved] });
    deleteMock.mockResolvedValue({ status: "ok", data: null });
    renderProfiles();

    await waitFor(() => {
      expect(screen.getByTestId("saved-QA baseline")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("delete-saved-QA baseline"));

    await waitFor(() => {
      expect(deleteMock).toHaveBeenCalledWith("QA baseline");
    });
  });
});
