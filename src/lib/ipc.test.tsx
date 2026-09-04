import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

import { usePing, IpcCallError } from "./ipc";
import { commands } from "./bindings";

vi.mock("./bindings", () => ({
  commands: { ping: vi.fn() },
}));

const pingMock = vi.mocked(commands.ping);

function wrapper({ children }: { children: ReactNode }): React.JSX.Element {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe("usePing", () => {
  beforeEach(() => {
    pingMock.mockReset();
  });

  it("surfaces the message and version from an ok result", async () => {
    pingMock.mockResolvedValue({
      status: "ok",
      data: { message: "pong, EmuManager", version: "9.9.9" },
    });

    const { result } = renderHook(() => usePing("EmuManager"), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });
    expect(result.current.data).toEqual({ message: "pong, EmuManager", version: "9.9.9" });
    expect(pingMock).toHaveBeenCalledWith("EmuManager");
  });

  it("rejects with the backend IpcError on an error result", async () => {
    pingMock.mockResolvedValue({
      status: "error",
      error: { code: "invalid", message: "name must not be blank", details: null },
    });

    const { result } = renderHook(() => usePing(""), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
    expect(result.current.error).toBeInstanceOf(IpcCallError);
    expect(result.current.error?.ipc).toMatchObject({
      code: "invalid",
      message: "name must not be blank",
    });
  });
});
