import { useState } from "react";

import { Screen, Placeholder } from "../components/Screen";
import type { ComponentInfo, HostReportDto } from "../lib/bindings";
import {
  useBootstrapProgress,
  useBootstrapToolchain,
  useComponents,
  useHostReport,
  useInstallComponent,
  useRunHelper,
  useUninstallComponent,
} from "../lib/ipc";
import type { BootstrapRunState } from "../lib/ipc";

/** The `sdkmanager` package path of the command-line tools — never removable from the UI. */
const CMDLINE_TOOLS_ID = "cmdline-tools;latest";

function formatSize(bytes: number): string {
  if (bytes <= 0) {
    return "—";
  }
  const mb = bytes / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb).toFixed(0)} MB`;
}

function ComponentRow({
  component,
  busy,
}: {
  component: ComponentInfo;
  busy: boolean;
}): React.JSX.Element {
  const install = useInstallComponent();
  const uninstall = useUninstallComponent();
  const [confirming, setConfirming] = useState(false);

  // The backend only removes components it installed itself, and never the command-line tools.
  const removable =
    component.installed && component.source === "app-managed" && component.id !== CMDLINE_TOOLS_ID;

  return (
    <li
      data-testid={`component-${component.id}`}
      className="flex items-center justify-between gap-3 rounded-card border border-border-default bg-surface px-3.5 py-3 text-[13px]"
    >
      <div className="flex flex-col gap-0.5">
        <span>{component.name}</span>
        <span className="text-[11px] text-muted">
          {component.version} · {formatSize(component.sizeBytes)}
        </span>
        {uninstall.error ? (
          <span data-testid={`uninstall-error-${component.id}`} className="text-[11px] text-danger">
            {uninstall.error.ipc.message}
          </span>
        ) : null}
      </div>
      {component.installed ? (
        <div className="flex shrink-0 items-center gap-2">
          <span className="text-[12px] text-running">
            installed{component.source ? ` (${component.source})` : ""}
          </span>
          {removable && !confirming ? (
            <button
              type="button"
              data-testid={`uninstall-${component.id}`}
              disabled={busy || uninstall.isPending}
              onClick={() => {
                setConfirming(true);
              }}
              className="rounded-md border border-border-default px-2.5 py-1 text-[11px] text-muted hover:text-danger disabled:opacity-50"
            >
              Uninstall
            </button>
          ) : null}
          {removable && confirming ? (
            <>
              <button
                type="button"
                data-testid={`uninstall-confirm-${component.id}`}
                disabled={uninstall.isPending}
                onClick={() => {
                  uninstall.mutate(component.id, {
                    onSettled: () => {
                      setConfirming(false);
                    },
                  });
                }}
                className="rounded-md border border-danger px-2.5 py-1 text-[11px] font-medium text-danger disabled:opacity-50"
              >
                {uninstall.isPending ? "Removing…" : "Confirm remove"}
              </button>
              <button
                type="button"
                data-testid={`uninstall-cancel-${component.id}`}
                disabled={uninstall.isPending}
                onClick={() => {
                  setConfirming(false);
                }}
                className="text-[11px] text-muted underline disabled:opacity-50"
              >
                Cancel
              </button>
            </>
          ) : null}
        </div>
      ) : (
        <button
          type="button"
          data-testid={`install-${component.id}`}
          disabled={busy || install.isPending}
          onClick={() => {
            install.mutate(component.id);
          }}
          className="shrink-0 rounded-md border border-primary px-3 py-1.5 text-[12px] font-medium text-primary disabled:opacity-50"
        >
          {install.isPending ? "Installing…" : "Install"}
        </button>
      )}
    </li>
  );
}

function RunProgress({
  run,
  extraError,
}: {
  run: BootstrapRunState;
  extraError: string | null;
}): React.JSX.Element | null {
  const errorMessage = run.error ?? extraError;
  if (!run.running && run.log.length === 0 && !errorMessage) {
    return null;
  }

  let heading: string;
  if (run.running) {
    heading = run.phase ?? "Installing…";
  } else if (errorMessage) {
    heading = "Install failed";
  } else {
    heading = "Install complete";
  }

  return (
    <div className="rounded-card border border-border-default bg-surface p-3.5 text-[12.5px]">
      <div className="mb-2 flex items-center justify-between">
        <span className="font-semibold text-ink">{heading}</span>
        {run.pct !== null ? <span className="text-muted">{run.pct}%</span> : null}
      </div>
      {run.running ? (
        <div
          data-testid="bootstrap-bar"
          data-indeterminate={run.pct === null}
          className="mb-2 h-1.5 overflow-hidden rounded-full bg-panel"
        >
          {run.pct !== null ? (
            <div
              className="h-full rounded-full bg-primary transition-[width] duration-300"
              style={{ width: `${String(run.pct)}%` }}
            />
          ) : (
            <div className="h-full w-full animate-pulse rounded-full bg-primary/60" />
          )}
        </div>
      ) : null}
      {errorMessage ? <p className="mb-2 text-danger">{errorMessage}</p> : null}
      <div
        data-testid="bootstrap-log"
        className="max-h-40 overflow-y-auto rounded-md bg-panel p-2 font-mono text-[11px] text-muted"
      >
        {run.log.length === 0 ? (
          <p>Waiting for output…</p>
        ) : (
          run.log.map((line, i) => <p key={`${String(i)}-${line}`}>{line}</p>)
        )}
      </div>
    </div>
  );
}

const VERDICT_STYLE: Record<string, string> = {
  canAccelerate: "border-running text-running",
  degraded: "border-attention text-attention",
  cannotRun: "border-danger text-danger",
};

function Tile({ label, value }: { label: string; value: string }): React.JSX.Element {
  return (
    <div className="rounded-card border border-border-default bg-surface px-3 py-2">
      <div className="text-[10.5px] uppercase tracking-wide text-faint">{label}</div>
      <div className="text-[12.5px] text-ink">{value}</div>
    </div>
  );
}

function HostPanel({ report }: { report: HostReportDto }): React.JSX.Element {
  const runHelper = useRunHelper();
  const gb = (mb: number): string => (mb > 0 ? `${(mb / 1024).toFixed(1)} GB` : "unknown");

  return (
    <section data-testid="host-panel" className="flex flex-col gap-3">
      <h2 className="text-[13px] font-semibold text-ink">Host</h2>
      <div
        data-testid="host-verdict"
        data-verdict={report.verdict}
        className={`rounded-card border bg-surface px-3.5 py-2.5 text-[12.5px] ${
          VERDICT_STYLE[report.verdict] ?? "border-border-default text-muted"
        }`}
      >
        {report.verdict === "canAccelerate"
          ? "Ready — emulators will run with hardware acceleration."
          : report.verdictReason}
      </div>

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        <Tile label="Virtualization" value={report.virtualization} />
        <Tile
          label="Accelerator"
          value={`${report.acceleratorKind} · ${report.acceleratorStatus}`}
        />
        <Tile label="RAM" value={gb(report.ramMb)} />
        <Tile label="Disk free" value={gb(report.diskFreeMb)} />
      </div>

      {report.fixes.length > 0 ? (
        <ul className="flex flex-col gap-2" data-testid="host-fixes">
          {report.fixes.map((fix) => (
            <li
              key={fix.id}
              data-testid={`fix-${fix.id}`}
              className="rounded-card border border-border-default bg-surface px-3.5 py-3 text-[12.5px]"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-medium text-ink">{fix.title}</span>
                {fix.scriptable ? (
                  <button
                    type="button"
                    data-testid={`fix-run-${fix.id}`}
                    disabled={runHelper.isPending}
                    onClick={() => {
                      runHelper.mutate(fix.id);
                    }}
                    className="rounded-md bg-primary px-3 py-1.5 text-[12px] font-medium text-white disabled:opacity-50"
                  >
                    Fix it
                  </button>
                ) : (
                  <span className="text-[11px] text-faint">manual</span>
                )}
              </div>
              <p className="mt-1 text-muted">{fix.description}</p>
              {fix.needsReboot ? (
                <p className="mt-1 text-attention">A reboot is required after this fix.</p>
              ) : null}
            </li>
          ))}
        </ul>
      ) : null}

      {runHelper.data ? (
        <p data-testid="fix-result" className="text-[12px] text-muted">
          {runHelper.data.message}
        </p>
      ) : null}
      {runHelper.error ? (
        <p className="text-[12px] text-danger">
          {runHelper.error.ipc.code === "cancelled"
            ? "The administrator prompt was dismissed — nothing changed."
            : runHelper.error.ipc.message}
        </p>
      ) : null}
    </section>
  );
}

export function Dependencies(): React.JSX.Element {
  const { data: components, error, isPending } = useComponents();
  const host = useHostReport();
  const bootstrap = useBootstrapToolchain();
  const { state: run, reset } = useBootstrapProgress();

  const handleInstall = (): void => {
    reset();
    bootstrap.mutate();
  };

  const allInstalled = components?.every((c) => c.installed) ?? false;
  const installing = bootstrap.isPending || run.running;
  const missingCount = components?.filter((c) => !c.installed).length ?? 0;

  let body: React.JSX.Element;
  if (isPending) {
    body = <Placeholder>Checking installed components…</Placeholder>;
  } else if (error) {
    body = <Placeholder>Could not load the component catalog: {error.ipc.message}</Placeholder>;
  } else {
    body = (
      <ul className="grid gap-2 sm:grid-cols-2">
        {components.map((component) => (
          <ComponentRow key={component.id} component={component} busy={installing} />
        ))}
      </ul>
    );
  }

  return (
    <Screen
      title="Dependencies & SDK"
      actions={
        missingCount > 1 && !allInstalled ? (
          <button
            type="button"
            data-testid="install-button"
            onClick={handleInstall}
            disabled={installing || isPending}
            className="rounded-md bg-primary px-3.5 py-2 text-[13px] font-medium text-white disabled:opacity-50"
          >
            {installing ? "Installing…" : `Install all (${String(missingCount)})`}
          </button>
        ) : undefined
      }
    >
      {body}

      <RunProgress run={run} extraError={bootstrap.error?.ipc.message ?? null} />

      {host.data ? (
        <HostPanel report={host.data} />
      ) : host.error ? (
        <Placeholder>Could not probe the host: {host.error.ipc.message}</Placeholder>
      ) : null}

      <Placeholder>
        Components resolve live from Google&apos;s repository and install into Emulator
        Studio&apos;s own data directory — no Android Studio, no terminal. A component already found
        on this machine (an existing Android Studio SDK, for example) is reused, never
        re-downloaded.
      </Placeholder>
    </Screen>
  );
}
