// Typed wrappers over the generated `tauri-specta` bindings.
//
// `bindings.ts` is generated (`just bindings`) and returns tauri-specta's
// `{ status: "ok" | "error" }` envelope. This module unwraps it: the promise
// resolves with the value, or rejects with an `IpcCallError` — a real `Error`
// (so stack traces, error boundaries and TanStack Query all behave) that
// carries the backend's structured `IpcError` on `.ipc`.

import { useEffect, useState } from "react";

import { useMutation, useQuery, useQueryClient, type UseQueryResult } from "@tanstack/react-query";
import { save } from "@tauri-apps/plugin-dialog";

import {
  commands,
  events,
  type AppInfo,
  type BootstrapProgressKind,
  type ComponentInfo,
  type CreateEmulatorRequest,
  type DeviceFactsDto,
  type DeviceInfo,
  type EmulatorDetail,
  type EmulatorInfo,
  type EmulatorJobKind,
  type HelperOutcomeDto,
  type HostReportDto,
  type ImageInfo,
  type IpcError,
  type Pong,
  type ProfileInspection,
  type ProfileSummary,
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

async function installComponent(componentId: string): Promise<undefined> {
  unwrap(await commands.installComponent(componentId));
  return undefined;
}

/** Mutation hook for a single component's "Install" button. Refetches {@link useComponents}. */
export function useInstallComponent() {
  const queryClient = useQueryClient();
  return useMutation<undefined, IpcCallError, string>({
    mutationFn: installComponent,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: COMPONENTS_QUERY_KEY });
    },
  });
}

async function uninstallComponent(componentId: string): Promise<undefined> {
  unwrap(await commands.uninstallComponent(componentId));
  return undefined;
}

/**
 * Mutation hook for a component row's "Uninstall" button — the mirror of
 * {@link useInstallComponent}. Refetches {@link useComponents} on success so the row flips back
 * to its "Install" state.
 */
export function useUninstallComponent() {
  const queryClient = useQueryClient();
  return useMutation<undefined, IpcCallError, string>({
    mutationFn: uninstallComponent,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: COMPONENTS_QUERY_KEY });
    },
  });
}

/** Static app metadata (version, licence, features) for the About screen. Never fails. */
export function useAppInfo(): UseQueryResult<AppInfo, IpcCallError> {
  return useQuery<AppInfo, IpcCallError>({
    queryKey: ["app-info"],
    queryFn: () => commands.appInfo(),
    staleTime: Infinity,
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
    case "progress": {
      // A new phase that carries no percentage (e.g. the `sdkmanager` download step, whose
      // output format we deliberately don't parse) means "percentage unknown from here" — drop
      // the stale one so the UI shows an indeterminate bar rather than a frozen number.
      const phaseChanged = kind.phase !== null && kind.phase !== prev.phase;
      return {
        ...prev,
        running: true,
        phase: kind.phase ?? prev.phase,
        pct: kind.pct ?? (phaseChanged ? null : prev.pct),
      };
    }
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
// M3 — reconcile (task 0020) + detail panel / log console (task 0021)
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

function emulatorDetailKey(id: string): [string, string] {
  return ["emulator-detail", id];
}

async function emulatorDetail(id: string): Promise<EmulatorDetail> {
  return unwrap(await commands.emulatorDetail(id));
}

/** TanStack Query hook for the detail panel. Polls every 4s so live state stays fresh. */
export function useEmulatorDetail(id: string): UseQueryResult<EmulatorDetail, IpcCallError> {
  return useQuery<EmulatorDetail, IpcCallError>({
    queryKey: emulatorDetailKey(id),
    queryFn: () => emulatorDetail(id),
    refetchInterval: 4000,
  });
}

async function emulatorLogTail(id: string): Promise<string[]> {
  return unwrap(await commands.emulatorLogTail(id, 500));
}

/** History tail of an emulator's launch log. Live lines arrive on `job://emulator` separately. */
export function useEmulatorLogTail(id: string): UseQueryResult<string[], IpcCallError> {
  return useQuery<string[], IpcCallError>({
    queryKey: ["emulator-log", id],
    queryFn: () => emulatorLogTail(id),
  });
}

/** Invalidate the emulator list and, when given, one emulator's detail. */
function useEmulatorInvalidator(): (id?: string) => void {
  const queryClient = useQueryClient();
  return (id?: string) => {
    void queryClient.invalidateQueries({ queryKey: EMULATORS_QUERY_KEY });
    if (id !== undefined) {
      void queryClient.invalidateQueries({ queryKey: emulatorDetailKey(id) });
    }
  };
}

async function renameEmulator(vars: { id: string; displayName: string }): Promise<undefined> {
  unwrap(await commands.renameEmulator(vars.id, vars.displayName));
  return undefined;
}

/** Mutation hook for the detail panel's inline rename. */
export function useRenameEmulator() {
  const invalidate = useEmulatorInvalidator();
  return useMutation<undefined, IpcCallError, { id: string; displayName: string }>({
    mutationFn: renameEmulator,
    onSuccess: (_data, vars) => {
      invalidate(vars.id);
    },
  });
}

interface EditHardwareVars {
  id: string;
  ramMb: number;
  storageMb: number;
  graphics: string;
  deviceFrame: boolean;
}

async function editHardware(vars: EditHardwareVars): Promise<undefined> {
  unwrap(
    await commands.editHardware(
      vars.id,
      vars.ramMb,
      vars.storageMb,
      vars.graphics,
      vars.deviceFrame,
    ),
  );
  return undefined;
}

/** Mutation hook for the detail panel's hardware form. */
export function useEditHardware() {
  const invalidate = useEmulatorInvalidator();
  return useMutation<undefined, IpcCallError, EditHardwareVars>({
    mutationFn: editHardware,
    onSuccess: (_data, vars) => {
      invalidate(vars.id);
    },
  });
}

async function deleteEmulator(vars: { id: string; wipe: boolean }): Promise<undefined> {
  unwrap(await commands.deleteEmulator(vars.id, vars.wipe));
  return undefined;
}

/** Mutation hook for the detail panel's "Delete" (with an "also remove the AVD" choice). */
export function useDeleteEmulator() {
  const invalidate = useEmulatorInvalidator();
  return useMutation<undefined, IpcCallError, { id: string; wipe: boolean }>({
    mutationFn: deleteEmulator,
    onSuccess: () => {
      invalidate();
    },
  });
}

async function wipeEmulatorData(id: string): Promise<undefined> {
  unwrap(await commands.wipeEmulatorData(id));
  return undefined;
}

/** Mutation hook for the detail panel's "Wipe data". */
export function useWipeEmulatorData() {
  const invalidate = useEmulatorInvalidator();
  return useMutation<undefined, IpcCallError, string>({
    mutationFn: wipeEmulatorData,
    onSuccess: (_data, id) => {
      invalidate(id);
    },
  });
}

/** Reveal a path in the OS file manager. Rejects with {@link IpcCallError}. */
export async function revealPath(path: string): Promise<void> {
  unwrap(await commands.revealPath(path));
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
    case "progress": {
      // Same rule as the bootstrap reducer: a new phase without a percentage clears the old one.
      const phaseChanged = kind.phase !== null && kind.phase !== prev.phase;
      return {
        ...prev,
        running: true,
        phase: kind.phase ?? prev.phase,
        pct: kind.pct ?? (phaseChanged ? null : prev.pct),
      };
    }
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

// ---------------------------------------------------------------------------
// M4 — profiles (task 0024)
// ---------------------------------------------------------------------------

const PROFILES_QUERY_KEY = ["profiles"];

async function inspectProfile(bytes: number[]): Promise<ProfileInspection> {
  return unwrap(await commands.inspectProfile(bytes));
}

/** Mutation hook for a dropped `.emuprofile` — parse + validate + resolve into a preview. */
export function useInspectProfile() {
  return useMutation<ProfileInspection, IpcCallError, number[]>({
    mutationFn: inspectProfile,
  });
}

async function applyProfile(vars: { bytes: number[]; launch: boolean }): Promise<string> {
  return unwrap(await commands.applyProfile(vars.bytes, vars.launch));
}

/** Mutation hook for "Apply" / "Apply & launch" — creates the emulator from the recipe. */
export function useApplyProfile() {
  const queryClient = useQueryClient();
  return useMutation<string, IpcCallError, { bytes: number[]; launch: boolean }>({
    mutationFn: applyProfile,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: EMULATORS_QUERY_KEY });
    },
  });
}

async function exportProfile(id: string): Promise<string> {
  return unwrap(await commands.exportProfile(id));
}

/** Mutation hook for the detail panel's "Export profile" — returns the `.emuprofile` JSON. */
export function useExportProfile() {
  return useMutation<string, IpcCallError, string>({ mutationFn: exportProfile });
}

interface ExportProfileVars {
  id: string;
  /** Filename stem offered in the Save dialog (usually the AVD name). */
  suggestedName: string;
}

/**
 * Open the native Save dialog, then write the emulator's `.emuprofile` to the chosen path.
 * Resolves to that path, or `null` if the user cancelled the dialog.
 */
async function exportProfileToFile(vars: ExportProfileVars): Promise<string | null> {
  const path = await save({
    defaultPath: `${vars.suggestedName}.emuprofile`,
    filters: [{ name: "Emulator profile", extensions: ["emuprofile"] }],
  });
  if (path === null) {
    return null;
  }
  unwrap(await commands.exportProfileToPath(vars.id, path));
  return path;
}

/**
 * Mutation hook: pick a location, then export the emulator's `.emuprofile` there — the mirror of
 * the Profiles screen's drag-in import. `data` is `null` when the Save dialog was cancelled.
 */
export function useExportProfileToFile() {
  return useMutation<string | null, IpcCallError, ExportProfileVars>({
    mutationFn: exportProfileToFile,
  });
}

async function listProfiles(): Promise<ProfileSummary[]> {
  return unwrap(await commands.listProfiles());
}

/** TanStack Query hook for the saved-profiles list. */
export function useProfiles(): UseQueryResult<ProfileSummary[], IpcCallError> {
  return useQuery<ProfileSummary[], IpcCallError>({
    queryKey: PROFILES_QUERY_KEY,
    queryFn: listProfiles,
  });
}

async function saveProfile(bytes: number[]): Promise<undefined> {
  unwrap(await commands.saveProfile(bytes));
  return undefined;
}

/** Mutation hook for "Save as profile" (Create wizard) and "Save to library" (Profiles import). */
export function useSaveProfile() {
  const queryClient = useQueryClient();
  return useMutation<undefined, IpcCallError, number[]>({
    mutationFn: saveProfile,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: PROFILES_QUERY_KEY });
    },
  });
}

async function deleteProfile(name: string): Promise<undefined> {
  unwrap(await commands.deleteProfile(name));
  return undefined;
}

/** Mutation hook for a saved-profiles row's "Delete". */
export function useDeleteProfile() {
  const queryClient = useQueryClient();
  return useMutation<undefined, IpcCallError, string>({
    mutationFn: deleteProfile,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: PROFILES_QUERY_KEY });
    },
  });
}

/** The JSON body of a saved profile — feed it (as bytes) to {@link useApplyProfile}. */
export async function getSavedProfile(name: string): Promise<string> {
  return unwrap(await commands.getSavedProfile(name));
}

// ---------------------------------------------------------------------------
// M5 — host readiness (task 0027)
// ---------------------------------------------------------------------------

const HOST_REPORT_QUERY_KEY = ["host-report"];

async function probeHost(): Promise<HostReportDto> {
  return unwrap(await commands.probeHost());
}

/** The host-readiness report. Refetched every 30s — it shells out, so not on the 4s cadence. */
export function useHostReport(): UseQueryResult<HostReportDto, IpcCallError> {
  return useQuery<HostReportDto, IpcCallError>({
    queryKey: HOST_REPORT_QUERY_KEY,
    queryFn: probeHost,
    refetchInterval: 30000,
  });
}

async function runHelper(fixId: string): Promise<HelperOutcomeDto> {
  return unwrap(await commands.runHelper(fixId));
}

/** Mutation hook for a "Fix it" button. Re-probes the host on settle (Windows returns a
 * synthetic outcome, so the report is the source of truth). */
export function useRunHelper() {
  const queryClient = useQueryClient();
  return useMutation<HelperOutcomeDto, IpcCallError, string>({
    mutationFn: runHelper,
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: HOST_REPORT_QUERY_KEY });
    },
  });
}

async function exportDiagnostics(): Promise<string> {
  return unwrap(await commands.exportDiagnostics());
}

/** Write a redacted diagnostics zip (log tail + host report + versions + emulators) to the data
 * dir and resolve to its path — for attaching to a bug report. */
export function useExportDiagnostics() {
  return useMutation<string, IpcCallError>({ mutationFn: exportDiagnostics });
}

// ---------------------------------------------------------------------------
// Device inspector (task 0038)
// ---------------------------------------------------------------------------

/** One parsed `adb logcat -v threadtime` line. `level` is `?` and `tag` empty when the line
 * doesn't match the threadtime format (a synthetic marker, or a multi-line payload). */
export interface LogcatLine {
  raw: string;
  level: string;
  tag: string;
  message: string;
}

/** `MM-DD HH:MM:SS.mmm  PID  TID L TAG: message` — the documented `threadtime` format
 * (https://developer.android.com/tools/logcat#outputFormat). */
const THREADTIME = /^\d\d-\d\d \d\d:\d\d:\d\d\.\d+\s+\d+\s+\d+\s+([VDIWEF])\s+(.+?):\s?(.*)$/;

/** Ordering used by the "minimum level" filter, mirroring Android Studio's Logcat. */
export const LOGCAT_LEVELS = ["V", "D", "I", "W", "E"] as const;
const LEVEL_RANK: Record<string, number> = { V: 0, D: 1, I: 2, W: 3, E: 4, F: 5 };

function parseLogcatLine(raw: string): LogcatLine {
  const m = THREADTIME.exec(raw);
  if (!m) {
    return { raw, level: "?", tag: "", message: raw };
  }
  return { raw, level: m[1] ?? "?", tag: (m[2] ?? "").trim(), message: m[3] ?? "" };
}

/** Filter parsed lines by a minimum level, a tag substring and a free-text substring — the
 * same three knobs Android Studio's Logcat exposes. An unparsed line (`level === "?"`) always
 * passes the level gate so nothing is silently dropped. */
export function filterLogcat(
  lines: LogcatLine[],
  opts: { minLevel: string; tag: string; text: string },
): LogcatLine[] {
  const min = LEVEL_RANK[opts.minLevel] ?? 0;
  const tag = opts.tag.trim().toLowerCase();
  const text = opts.text.trim().toLowerCase();
  return lines.filter((l) => {
    if (l.level !== "?" && (LEVEL_RANK[l.level] ?? 0) < min) {
      return false;
    }
    if (tag && !l.tag.toLowerCase().includes(tag)) {
      return false;
    }
    if (text && !l.raw.toLowerCase().includes(text)) {
      return false;
    }
    return true;
  });
}

async function startLogcat(id: string): Promise<undefined> {
  unwrap(await commands.startLogcat(id));
  return undefined;
}

async function stopLogcat(id: string): Promise<undefined> {
  unwrap(await commands.stopLogcat(id));
  return undefined;
}

/** Start / stop the backend `adb logcat` stream for an emulator. */
export function useStartLogcat() {
  return useMutation<undefined, IpcCallError, string>({ mutationFn: startLogcat });
}
export function useStopLogcat() {
  return useMutation<undefined, IpcCallError, string>({ mutationFn: stopLogcat });
}

/** Accumulate `device://log` lines for one emulator (capped), with a `clear`. Subscribing does
 * not start the stream — call {@link useStartLogcat} for that. */
export function useLogcatStream(id: string): { lines: LogcatLine[]; clear: () => void } {
  const [lines, setLines] = useState<LogcatLine[]>([]);

  useEffect(() => {
    setLines([]);
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void events.deviceLog
      .listen((event) => {
        if (event.payload.id !== id) {
          return;
        }
        setLines((prev) => {
          const next = [...prev, parseLogcatLine(event.payload.line)];
          return next.length > 5000 ? next.slice(next.length - 5000) : next;
        });
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
  }, [id]);

  return {
    lines,
    clear: () => {
      setLines([]);
    },
  };
}

async function deviceFacts(id: string): Promise<DeviceFactsDto> {
  return unwrap(await commands.deviceFacts(id));
}

/** Poll a running emulator's model / Android version / battery / storage every 5 s. */
export function useDeviceFacts(
  id: string,
  enabled: boolean,
): UseQueryResult<DeviceFactsDto, IpcCallError> {
  return useQuery<DeviceFactsDto, IpcCallError>({
    queryKey: ["device-facts", id],
    queryFn: () => deviceFacts(id),
    enabled,
    refetchInterval: 5000,
  });
}
