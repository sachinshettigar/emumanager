import { Link } from "react-router-dom";

import { Placeholder, Screen } from "../components/Screen";
import type { EmulatorInfo } from "../lib/bindings";
import { useEmulators, useLaunchEmulator, usePing, useStopEmulator } from "../lib/ipc";

/** Backend liveness line: proves the typed IPC seam is wired end to end. */
function BackendStatus(): React.JSX.Element {
  const { data, error, isPending } = usePing("EmuManager");

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
  const isRunningOrBooting = emulator.state === "running" || emulator.state === "booting";
  const busy = launch.isPending || stop.isPending;

  return (
    <li
      data-testid={`emulator-${emulator.id}`}
      className="flex items-center justify-between rounded-card border border-border-default bg-surface px-3.5 py-3 text-[13px]"
    >
      <div className="flex flex-col gap-0.5">
        <span>{emulator.displayName}</span>
        <span className="text-[11px] text-muted">
          {emulator.avdName}
          {emulator.adbSerial ? ` · ${emulator.adbSerial}` : ""}
        </span>
      </div>
      <div className="flex items-center gap-3">
        <span
          data-testid={`emulator-state-${emulator.id}`}
          className={`text-[12px] ${STATE_STYLES[emulator.state] ?? "text-muted"}`}
        >
          {emulator.state}
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
            disabled={busy}
            onClick={() => {
              launch.mutate(emulator.id);
            }}
            className="rounded-md bg-primary px-3 py-1.5 text-[12px] font-medium text-white disabled:opacity-50"
          >
            {launch.isPending ? "Launching…" : "Launch"}
          </button>
        )}
      </div>
    </li>
  );
}

export function Dashboard(): React.JSX.Element {
  const { data: emulators, error, isPending } = useEmulators();

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
        <Link
          to="/create"
          data-testid="create-link"
          className="rounded-md bg-primary px-3.5 py-2 text-[13px] font-medium text-white"
        >
          New emulator
        </Link>
      }
    >
      <div className="flex flex-wrap items-center gap-3.5 rounded-card border border-border-default bg-surface px-3.5 py-3 text-[12.5px] text-muted">
        <span className="font-semibold text-ink">Host</span>
        <span>detection lands in M5</span>
        <span className="text-faint">|</span>
        <BackendStatus />
      </div>

      {body}
    </Screen>
  );
}
