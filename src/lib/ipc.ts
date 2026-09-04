// Typed wrappers over the generated `tauri-specta` bindings.
//
// `bindings.ts` is generated (`just bindings`) and returns tauri-specta's
// `{ status: "ok" | "error" }` envelope. This module unwraps it: the promise
// resolves with the value, or rejects with an `IpcCallError` — a real `Error`
// (so stack traces, error boundaries and TanStack Query all behave) that
// carries the backend's structured `IpcError` on `.ipc`.

import { useEffect, useState } from "react";

import { useMutation, useQuery, useQueryClient, type UseQueryResult } from "@tanstack/react-query";

import {
  commands,
  events,
  type BootstrapProgressKind,
  type ComponentInfo,
  type IpcError,
  type Pong,
} from "./bindings";

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
async function ping(name: string): Promise<Pong> {
  return unwrap(await commands.ping(name));
}

/** TanStack Query hook for {@link ping}. `error` is typed as {@link IpcCallError}. */
export function usePing(name: string): UseQueryResult<Pong, IpcCallError> {
  return useQuery<Pong, IpcCallError>({
    queryKey: ["ping", name],
    queryFn: () => ping(name),
  });
}

const COMPONENTS_QUERY_KEY = ["components"];

/** The real M1 component catalog merged with real installed state. Rejects with {@link IpcCallError}. */
async function listComponents(): Promise<ComponentInfo[]> {
  return unwrap(await commands.listComponents());
}

/** TanStack Query hook for {@link listComponents} — what the Dependencies screen lists. */
export function useComponents(): UseQueryResult<ComponentInfo[], IpcCallError> {
  return useQuery<ComponentInfo[], IpcCallError>({
    queryKey: COMPONENTS_QUERY_KEY,
    queryFn: listComponents,
  });
}

async function bootstrapToolchain(): Promise<undefined> {
  unwrap(await commands.bootstrapToolchain());
  return undefined;
}

/**
 * Mutation hook for the Dependencies screen's "Install" action. Re-fetches
 * {@link useComponents} on success so newly-installed components show up without a manual
 * refresh.
 *
 * No explicit `UseMutationResult<...>` return-type annotation: `bootstrapToolchain` takes no
 * arguments, and every way of spelling that generic (`void`, or a `TVariables` default that
 * disagrees with what `useMutation` itself infers from `mutationFn`) fights either the linter's
 * `no-invalid-void-type` rule or the compiler — inference gets the same type without either.
 */
export function useBootstrapToolchain() {
  const queryClient = useQueryClient();
  return useMutation<undefined, IpcCallError>({
    mutationFn: bootstrapToolchain,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: COMPONENTS_QUERY_KEY });
    },
  });
}

/** Live state of the one toolchain-bootstrap job, built up from `job://bootstrap` events. */
export interface BootstrapRunState {
  /** `true` from the first event received until a `"done"` event arrives. */
  running: boolean;
  /** Most recent phase label, if any was reported. */
  phase: string | null;
  /** Most recent progress percentage, if any was reported. */
  pct: number | null;
  /** Every log line reported so far, oldest first. */
  log: string[];
  /** Set once a `"done"` event reports failure; cleared at the start of a new run. */
  error: string | null;
}

const INITIAL_BOOTSTRAP_RUN_STATE: BootstrapRunState = {
  running: false,
  phase: null,
  pct: null,
  log: [],
  error: null,
};

function applyBootstrapEvent(
  prev: BootstrapRunState,
  kind: BootstrapProgressKind,
): BootstrapRunState {
  switch (kind.type) {
    case "progress":
      return {
        ...prev,
        running: true,
        phase: kind.phase ?? prev.phase,
        pct: kind.pct ?? prev.pct,
      };
    case "log":
      return { ...prev, running: true, log: [...prev.log, kind.line] };
    case "done":
      return { ...prev, running: false, error: kind.error?.message ?? null };
  }
}

/**
 * {@link useBootstrapProgress}'s return value: the live state plus a way to clear it. Not
 * exported — nothing outside this module needs to name the type, only call the hook (`knip`
 * flags an exported-but-never-imported type as dead code, correctly).
 */
interface UseBootstrapProgressResult {
  state: BootstrapRunState;
  /** Clear the log/phase/pct/error before starting a new run — events otherwise accumulate
   * across runs (the log is meant to be a full tail of the *current* run, not all runs ever). */
  reset: () => void;
}

/** Subscribes to `job://bootstrap` for as long as the component using it is mounted. */
export function useBootstrapProgress(): UseBootstrapProgressResult {
  const [state, setState] = useState<BootstrapRunState>(INITIAL_BOOTSTRAP_RUN_STATE);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void events.jobBootstrap
      .listen((event) => {
        setState((prev) => applyBootstrapEvent(prev, event.payload.payload));
      })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return {
    state,
    reset: () => {
      setState(INITIAL_BOOTSTRAP_RUN_STATE);
    },
  };
}
