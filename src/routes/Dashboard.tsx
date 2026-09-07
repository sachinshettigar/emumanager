import { Link } from "react-router-dom";

import { Placeholder, Screen } from "../components/Screen";
import type { EmulatorInfo } from "../lib/bindings";
import {
  useComponents,
  useEmulators,
  useHostReport,
  useLaunchEmulator,
  usePing,
  useReconcileNow,
  useStopEmulator,
} from "../lib/ipc";

/** Compact first-run checklist. Hides itself once all three steps are done. */
function OnboardingChecklist(): React.JSX.Element | null {
  const components = useComponents();
  const host = useHostReport();
  const emulators = useEmulators();

  const sdkReady =
    (components.data?.every((c) => c.installed) ?? false) && components.data !== undefined;
  const hostReady = host.data !== undefined && host.data.verdict !== "cannotRun";
  const hasEmulator = (emulators.data?.length ?? 0) > 0;

  if (sdkReady && hostReady && hasEmulator) {
    return null;
  }

  const steps: { done: boolean; label: string; to: string; cta: string }[] = [
    {
      done: sdkReady,
      label: "Install the Android SDK",
      to: "/dependencies",
      cta: "Open Dependencies",
    },
    {
      done: hostReady,
      label: "Check host readiness",
      to: "/dependencies",
      cta: "See the host panel",
    },
    {
      done: hasEmulator,
      label: "Create your first emulator",
      to: "/create",
      cta: "Open the wizard",
    },
  ];

  return (
    <section
      data-testid="onboarding"
      className="flex flex-col gap-2 rounded-card border border-primary/40 bg-panel p-4 text-[13px]"
    >
      <h2 className="text-[13px] font-semibold text-ink">Get started</h2>
      <ol className="flex flex-col gap-2">
        {steps.map((s, i) => (
          <li key={s.to + s.label} className="flex items-center gap-2.5">
            <span
              className={`flex h-5 w-5 shrink-0 items-center justify-center rounded-full border text-[11px] ${
                s.done ? "border-running bg-running text-white" : "border-border-default text-muted"
              }`}
            >
              {s.done ? "✓" : i + 1}
            </span>
            <span className={s.done ? "text-muted line-through" : "text-ink"}>{s.label}</span>
            {!s.done ? (
              <Link
                to={s.to}
                data-testid={`onboarding-step-${String(i)}`}
                className="ml-auto text-[12px] text-primary underline"
              >
                {s.cta}
              </Link>
            ) : null}
          </li>
        ))}
      </ol>
    </section>
  );
}

/** Backend liveness line: proves the typed IPC seam is wired end to end. */
function BackendStatus(): React.JSX.Element {
  const { data, error, isPending } = usePing("Emulator Studio");

  let text: string;
  if (isPending) {
    text = "checking backend…";
  } else if (error) {
    text = `backend error: ${error.ipc.message}`;
  } else {
    text = `${data.message} from v${data.version}`;
  }

  return (
    <span data-testid="backend-status" className={error ? "text-danger" : undefined}>
      {text}
    </span>
  );
}

const STATE_STYLES: Record<string, string> = {
  running: "text-running",
  booting: "text-attention",
  stopped: "text-muted",
  error: "text-danger",
};

function EmulatorRow({ emulator }: { emulator: EmulatorInfo }): React.JSX.Element {
  const launch = useLaunchEmulator();
  const stop = useStopEmulator();
  const host = useHostReport();
  const isRunningOrBooting = emulator.state === "running" || emulator.state === "booting";
  const blocked = host.data?.verdict === "cannotRun";
  const busy = launch.isPending || stop.isPending;

  return (
    <li
      data-testid={`emulator-${emulator.id}`}
      className="flex items-center justify-between rounded-card border border-border-default bg-surface px-3.5 py-3 text-[13px]"
    >
      <div className="flex flex-col gap-0.5">
        <Link
          to={`/emulator/${emulator.id}`}
          data-testid={`detail-link-${emulator.id}`}
          className="hover:underline"
        >
          {emulator.displayName}
        </Link>
        <span className="text-[11px] text-muted">
          {emulator.avdName}
          {emulator.adbSerial ? ` · ${emulator.adbSerial}` : ""}
        </span>
      </div>
      <div className="flex items-center gap-3">
        <span
          data-testid={`emulator-state-${emulator.id}`}
          className={`flex items-center gap-1.5 text-[12px] ${STATE_STYLES[emulator.state] ?? "text-muted"}`}
        >
          {emulator.state === "booting" ? (
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-attention" aria-hidden />
          ) : null}
          {emulator.state === "booting" ? "booting…" : emulator.state}
        </span>
        {isRunningOrBooting ? (
          <button
            type="button"
            data-testid={`stop-${emulator.id}`}
            disabled={busy}
            onClick={() => {
              stop.mutate(emulator.id);
            }}
            className="rounded-md border border-border-default px-3 py-1.5 text-[12px] disabled:opacity-50"
          >
            Stop
          </button>
        ) : (
          <button
            type="button"
            data-testid={`launch-${emulator.id}`}
            disabled={busy || blocked}
            title={blocked ? host.data?.verdictReason : undefined}
            onClick={() => {
              launch.mutate(emulator.id);
            }}
            className="rounded-md bg-primary px-3 py-1.5 text-[12px] font-medium text-white disabled:opacity-50"
          >
            {launch.isPending ? "Launching…" : "Launch"}
          </button>
        )}
      </div>
      {blocked && !isRunningOrBooting ? (
        <span
          data-testid={`launch-blocked-${emulator.id}`}
          className="ml-3 shrink-0 text-[11px] text-danger"
        >
          {host.data?.verdictReason}
        </span>
      ) : null}
    </li>
  );
}

export function Dashboard(): React.JSX.Element {
  const { data: emulators, error, isPending } = useEmulators();
  const reconcile = useReconcileNow();

  let body: React.JSX.Element;
  if (isPending) {
    body = <Placeholder>Loading emulators…</Placeholder>;
  } else if (error) {
    body = <Placeholder>Could not load emulators: {error.ipc.message}</Placeholder>;
  } else if (emulators.length === 0) {
    body = (
      <Placeholder>
        No emulators yet. Install the SDK on the Dependencies screen, then create one — it will show
        up here with live Stopped / Booting / Running state.
      </Placeholder>
    );
  } else {
    body = (
      <ul className="flex flex-col gap-2">
        {emulators.map((emulator) => (
          <EmulatorRow key={emulator.id} emulator={emulator} />
        ))}
      </ul>
    );
  }

  return (
    <Screen
      title="My emulators"
      actions={
        <div className="flex items-center gap-2">
          <button
            type="button"
            data-testid="reconcile-button"
            disabled={reconcile.isPending}
            onClick={() => {
              reconcile.mutate();
            }}
            className="rounded-md border border-border-default px-3 py-2 text-[13px] disabled:opacity-50"
          >
            {reconcile.isPending ? "Refreshing…" : "Refresh"}
          </button>
          <Link
            to="/create"
            data-testid="create-link"
            className="rounded-md bg-primary px-3.5 py-2 text-[13px] font-medium text-white"
          >
            New emulator
          </Link>
        </div>
      }
    >
      <div className="flex flex-wrap items-center gap-3.5 rounded-card border border-border-default bg-surface px-3.5 py-3 text-[12.5px] text-muted">
        <span className="font-semibold text-ink">Host</span>
        <HostSummary />
        <span className="text-faint">|</span>
        <BackendStatus />
      </div>

      <OnboardingChecklist />

      {body}
    </Screen>
  );
}

/** One-line host verdict for the Dashboard strip; the full panel is on the Dependencies screen. */
function HostSummary(): React.JSX.Element {
  const host = useHostReport();
  if (!host.data) {
    return <span>{host.error ? "readiness check unavailable" : "checking readiness…"}</span>;
  }
  const label =
    host.data.verdict === "canAccelerate"
      ? "ready (hardware accelerated)"
      : host.data.verdict === "degraded"
        ? `degraded — ${host.data.verdictReason}`
        : `cannot run — ${host.data.verdictReason}`;
  const tone =
    host.data.verdict === "canAccelerate"
      ? "text-running"
      : host.data.verdict === "degraded"
        ? "text-attention"
        : "text-danger";
  return (
    <span className={tone}>
      {label}
      {host.data.verdict !== "canAccelerate" ? (
        <>
          {" "}
          <Link to="/dependencies" className="underline">
            fix
          </Link>
        </>
      ) : null}
    </span>
  );
}
