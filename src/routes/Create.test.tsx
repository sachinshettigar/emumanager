import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { Create } from "./Create";
import { commands, events } from "../lib/bindings";
import type { DeviceInfo, EmulatorJob, ImageInfo } from "../lib/bindings";

vi.mock("../lib/bindings", () => ({
  commands: {
    listDevices: vi.fn(),
    listImages: vi.fn(),
    createEmulator: vi.fn(),
  },
  events: {
    jobEmulator: { listen: vi.fn() },
  },
}));

const listDevicesMock = vi.mocked(commands.listDevices);
const listImagesMock = vi.mocked(commands.listImages);
const createEmulatorMock = vi.mocked(commands.createEmulator);
const listenMock = vi.mocked(events.jobEmulator.listen);

const PIXEL: DeviceInfo = {
  id: "pixel_6",
  name: "Pixel 6",
  oem: "Google",
  formFactor: "phone",
  ramMb: 2048,
  resolution: "1080 × 2400",
  densityDpi: 420,
  diagonalIn: 6.4,
  skin: "pixel_6",
};

const IMAGE: ImageInfo = {
  coord: "system-images;android-34;google_apis_playstore;x86_64",
  api: 34,
  androidVersion: "14",
  imageType: "google_apis_playstore",
  abi: "x86_64",
  revision: "14",
  downloadSizeBytes: 1_500_000_000,
  installed: false,
  hasPlayStore: true,
};

function renderCreate(): void {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <Create />
    </QueryClientProvider>,
  );
}

describe("Create wizard", () => {
  let emit: (payload: EmulatorJob) => void = () => {
    throw new Error("emit called before jobEmulator.listen resolved");
  };

  beforeEach(() => {
    listDevicesMock.mockReset();
    listImagesMock.mockReset();
    createEmulatorMock.mockReset();
    listenMock.mockReset();
    listenMock.mockImplementation((cb) => {
      emit = (payload) => {
        cb({ payload } as Parameters<typeof cb>[0]);
      };
      return Promise.resolve(() => {});
    });
    listDevicesMock.mockResolvedValue({ status: "ok", data: [PIXEL] });
    listImagesMock.mockResolvedValue({ status: "ok", data: [IMAGE] });
  });

  it("walks device → image → hardware → review → create, streaming job output", async () => {
    createEmulatorMock.mockResolvedValue({ status: "ok", data: "01J0NEWID" });
    renderCreate();

    // Step 1 — pick a device
    await waitFor(() => {
      expect(screen.getByTestId("device-pixel_6")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("device-pixel_6"));
    fireEvent.click(screen.getByTestId("next-button"));

    // Step 2 — pick an image (shows the download size since it isn't installed)
    await waitFor(() => {
      expect(
        screen.getByTestId("image-system-images;android-34;google_apis_playstore;x86_64"),
      ).toHaveTextContent("1.4 GB");
    });
    fireEvent.click(
      screen.getByTestId("image-system-images;android-34;google_apis_playstore;x86_64"),
    );
    fireEvent.click(screen.getByTestId("next-button"));

    // Step 3 — hardware
    expect(screen.getByTestId("name-input")).toBeInTheDocument();
    fireEvent.change(screen.getByTestId("name-input"), { target: { value: "My Pixel" } });
    fireEvent.click(screen.getByTestId("next-button"));

    // Step 4 — review + create
    expect(screen.getByTestId("review")).toHaveTextContent("My Pixel");
    fireEvent.click(screen.getByTestId("create-button"));

    await waitFor(() => {
      expect(createEmulatorMock).toHaveBeenCalledWith(
        expect.objectContaining({
          name: "My Pixel",
          deviceId: "pixel_6",
          imageCoord: "system-images;android-34;google_apis_playstore;x86_64",
          deviceFrame: true,
          launch: false,
        }),
      );
    });

    act(() => {
      emit({
        jobId: "create:My_Pixel",
        payload: { type: "log", line: "creating AVD My_Pixel" },
      });
    });
    await waitFor(() => {
      expect(screen.getByTestId("create-log")).toHaveTextContent("creating AVD My_Pixel");
    });
  });

  it("shows a device load failure", async () => {
    listDevicesMock.mockResolvedValue({
      status: "error",
      error: { code: "not_found", message: "install the SDK first", details: null },
    });
    renderCreate();
    await waitFor(() => {
      expect(screen.getByText(/install the SDK first/)).toBeInTheDocument();
    });
  });

  it("keeps Next disabled until a device is selected", async () => {
    renderCreate();
    await waitFor(() => {
      expect(screen.getByTestId("device-pixel_6")).toBeInTheDocument();
    });
    expect(screen.getByTestId("next-button")).toBeDisabled();
    fireEvent.click(screen.getByTestId("device-pixel_6"));
    expect(screen.getByTestId("next-button")).not.toBeDisabled();
  });

  it("marks installed images, sorts them first, and pre-selects one", async () => {
    const INSTALLED: ImageInfo = {
      coord: "system-images;android-33;google_apis;x86_64",
      api: 33,
      androidVersion: "13",
      imageType: "google_apis",
      abi: "x86_64",
      revision: "9",
      downloadSizeBytes: 1_200_000_000,
      installed: true,
      hasPlayStore: false,
    };
    listImagesMock.mockResolvedValue({ status: "ok", data: [IMAGE, INSTALLED] });
    renderCreate();

    await waitFor(() => {
      expect(screen.getByTestId("device-pixel_6")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId("device-pixel_6"));
    fireEvent.click(screen.getByTestId("next-button"));

    const installedCard = await screen.findByTestId(
      "image-system-images;android-33;google_apis;x86_64",
    );
    // Shown as an Android version, flagged installed, and already selected.
    expect(installedCard).toHaveTextContent("Android 13");
    expect(installedCard).toHaveTextContent("Installed");
    expect(installedCard).toHaveAttribute("data-selected", "true");
    // Installed sorts ahead of the larger, not-installed image.
    const cards = screen.getAllByTestId(/^image-/);
    expect(cards[0]).toBe(installedCard);
    // So Next is immediately enabled without another click.
    expect(screen.getByTestId("next-button")).not.toBeDisabled();
  });

  it("groups devices by form factor", async () => {
    const WEAR: DeviceInfo = {
      ...PIXEL,
      id: "wear_round",
      name: "Wear OS Small Round",
      formFactor: "wear",
      skin: "wearos_small_round",
    };
    listDevicesMock.mockResolvedValue({ status: "ok", data: [PIXEL, WEAR] });
    renderCreate();

    await waitFor(() => {
      expect(screen.getByTestId("device-group-phone")).toHaveTextContent("Phones (1)");
    });
    expect(screen.getByTestId("device-group-wear")).toHaveTextContent("Wear OS (1)");
    // A search narrows a group and drops the others.
    fireEvent.change(screen.getByTestId("device-search"), { target: { value: "wear" } });
    await waitFor(() => {
      expect(screen.queryByTestId("device-group-phone")).not.toBeInTheDocument();
    });
    expect(screen.getByTestId("device-wear_round")).toBeInTheDocument();
  });
});
