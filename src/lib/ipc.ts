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
  type CreateEmulatorRequest,
  type DeviceInfo,
  type EmulatorInfo,
  type EmulatorJobKind,
  type ImageInfo,
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

// ---------------------------------------------------------------------------
// M2 — devices, images, emulators (Create wizard + Dashboard)
// ---------------------------------------------------------------------------

const EMULATORS_QUERY_KEY = ["emulators"];

/** Google device profiles from the installed SDK. Rejects with {@link IpcCallError}. */
async function listDevices(): Promise<DeviceInfo[]> {
  return unwrap(await commands.listDevices());
}

/** TanStack Query hook for the Create wizard's device step. */
export function useDevices(): UseQueryResult<DeviceInfo[], IpcCallError> {
  return useQuery<DeviceInfo[], IpcCallError>({
    queryKey: ["devices"],
    queryFn: listDevices,
  });
}

/** Today's system-image catalog merged with local install state. */
async function listImages(): Promise<ImageInfo[]> {
  return unwrap(await commands.listImages());
}

/** TanStack Query hook for the Create wizard's image step. */
export function useImages(): UseQueryResult<ImageInfo[], IpcCallError> {
  return useQuery<ImageInfo[], IpcCallError>({
    queryKey: ["images"],
    queryFn: listImages,
  });
}

/** Every tracked emulator with its live run state. */
async function listEmulators(): Promise<EmulatorInfo[]> {
  return unwrap(await commands.listEmulators());
}

/**
 * TanStack Query hook for the Dashboard. Polls every 4s so a `Booting → Running` transition
 * shows up without a manual refresh.
 */
export function useEmulators(): UseQueryResult<EmulatorInfo[], IpcCallError> {
  return useQuery<EmulatorInfo[], IpcCallError>({
    queryKey: EMULATORS_QUERY_KEY,
    queryFn: listEmulators,
    refetchInterval: 4000,
  });
}

async function createEmulator(request: CreateEmulatorRequest): Promise<string> {
  return unwrap(await commands.createEmulator(request));
}

/**
 * Mutation hook for the Create wizard's "Create" / "Create & launch". Invalidates the emulator
 * list on success so the new row appears on the Dashboard.
 */
export function useCreateEmulator() {
  const queryClient = useQueryClient();
  return useMutation<string, IpcCallError, CreateEmulatorRequest>({
    mutationFn: createEmulator,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: EMULATORS_QUERY_KEY });
    },
  });
}

async function launchEmulator(id: string): Promise<undefined> {
  unwrap(await commands.launchEmulator(id));
  return undefined;
}

/** Mutation hook for a Dashboard row's "Launch" action. */
export function useLaunchEmulator() {
  const queryClient = useQueryClient();
  return useMutation<undefined, IpcCallError, string>({
    mutationFn: launchEmulator,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: EMULATORS_QUERY_KEY });
    },
  });
}

async function stopEmulator(id: string): Promise<undefined> {
  unwrap(await commands.stopEmulator(id));
  return undefined;
}

/** Mutation hook for a Dashboard row's "Stop" action. */
export function useStopEmulator() {
  const queryClient = useQueryClient();
  return useMutation<undefined, IpcCallError, string>({
    mutationFn: stopEmulator,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: EMULATORS_QUERY_KEY });
    },
  });
}

// ---------------------------------------------------------------------------
// M3 — reconcile (task 0020). The detail-panel hooks (rename / edit / delete /
// wipe / emulator_detail / revealPath) land in task 0021 with the panel that
// consumes them; their backend commands already exist in `bindings.ts`.
// ---------------------------------------------------------------------------

async function reconcileNow(): Promise<EmulatorInfo[]> {
  return unwrap(await commands.reconcileNow());
}

/** Mutation hook for the Dashboard's "Refresh" action — reconcile then refetch the list. */
export function useReconcileNow() {
  const queryClient = useQueryClient();
  return useMutation<EmulatorInfo[], IpcCallError>({
    mutationFn: reconcileNow,
    onSuccess: (list) => {
      queryClient.setQueryData(EMULATORS_QUERY_KEY, list);
    },
  });
}

/** Live state of the current emulator create/launch job, built up from `job://emulator` events. */
export interface EmulatorJobState {
  running: boolean;
  phase: string | null;
  pct: number | null;
  log: string[];
  error: string | null;
}

const INITIAL_EMULATOR_JOB_STATE: EmulatorJobState = {
  running: false,
  phase: null,
  pct: null,
  log: [],
  error: null,
};

function applyEmulatorJobEvent(prev: EmulatorJobState, kind: EmulatorJobKind): EmulatorJobState {
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

interface UseEmulatorJobResult {
  state: EmulatorJobState;
  /** Clear before starting a new run — events otherwise accumulate across runs. */
  reset: () => void;
}

/**
 * Subscribes to `job://emulator` while mounted. Only ever one such job is in flight at a time
 * (same simplification as `job://bootstrap`), so every event is applied regardless of its
 * `jobId`.
 */
export function useEmulatorJob(): UseEmulatorJobResult {
  const [state, setState] = useState<EmulatorJobState>(INITIAL_EMULATOR_JOB_STATE);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void events.jobEmulator
      .listen((event) => {
        setState((prev) => applyEmulatorJobEvent(prev, event.payload.payload));
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
      setState(INITIAL_EMULATOR_JOB_STATE);
    },
  };
}
