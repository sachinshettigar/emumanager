// Typed wrappers over the generated `tauri-specta` bindings.
//
// `bindings.ts` is generated (`just bindings`) and returns tauri-specta's
// `{ status: "ok" | "error" }` envelope. This module unwraps it: the promise
// resolves with the value, or rejects with an `IpcCallError` — a real `Error`
// (so stack traces, error boundaries and TanStack Query all behave) that
// carries the backend's structured `IpcError` on `.ipc`.

import { useQuery, type UseQueryResult } from "@tanstack/react-query";

import { commands, type IpcError, type Pong } from "./bindings";

export type { IpcError, Pong };

/** A failed IPC call. `ipc.code` is the stable machine string from the backend. */
export class IpcCallError extends Error {
  readonly ipc: IpcError;

  constructor(ipc: IpcError) {
    super(`[${ipc.code}] ${ipc.message}`);
    this.name = "IpcCallError";
    this.ipc = ipc;
  }
}

type Envelope<T> = { status: "ok"; data: T } | { status: "error"; error: IpcError };

/** Narrow the tauri-specta result envelope, rejecting with {@link IpcCallError}. */
function unwrap<T>(res: Envelope<T>): T {
  if (res.status === "error") {
    throw new IpcCallError(res.error);
  }
  return res.data;
}

/** Liveness check against the Rust backend. Rejects with an {@link IpcCallError}. */
export async function ping(name: string): Promise<Pong> {
  return unwrap(await commands.ping(name));
}

/** TanStack Query hook for {@link ping}. `error` is typed as {@link IpcCallError}. */
export function usePing(name: string): UseQueryResult<Pong, IpcCallError> {
  return useQuery<Pong, IpcCallError>({
    queryKey: ["ping", name],
    queryFn: () => ping(name),
  });
}
