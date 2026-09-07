import { useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";

import { Placeholder, Screen } from "../components/Screen";
import {
  revealPath,
  useDeleteEmulator,
  useEditHardware,
  useEmulatorDetail,
  useEmulatorJob,
  useEmulatorLogTail,
  useExportProfile,
  useHostReport,
  useLaunchEmulator,
  useRenameEmulator,
  useStopEmulator,
  useWipeEmulatorData,
} from "../lib/ipc";

const GRAPHICS_OPTIONS = ["auto", "host", "swiftshader"] as const;

/** Read-only config row. */
function Field({ label, value }: { label: string; value: React.ReactNode }): React.JSX.Element {
  return (
    <>
      <dt className="text-muted">{label}</dt>
      <dd className="tabular-nums">{value}</dd>
    </>
  );
}

export function EmulatorDetail(): React.JSX.Element {
  const { id = "" } = useParams();
  const detail = useEmulatorDetail(id);
  const logTail = useEmulatorLogTail(id);
  const job = useEmulatorJob();

  const rename = useRenameEmulator();
  const editHardware = useEditHardware();
  const del = useDeleteEmulator();
  const wipe = useWipeEmulatorData();
  const launch = useLaunchEmulator();
  const host = useHostReport();
  const stop = useStopEmulator();
  const exportProfile = useExportProfile();

  const [nameDraft, setNameDraft] = useState<string | null>(null);
  const [hw, setHw] = useState<{ ramMb: number; storageMb: number; graphics: string } | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [alsoRemoveAvd, setAlsoRemoveAvd] = useState(true);

  const d = detail.data;
  const effectiveName = nameDraft ?? d?.displayName ?? "";
  const effectiveHw = hw ?? {
    ramMb: d?.ramMb ?? 2048,
    storageMb: d?.storageMb ?? 6144,
    graphics: d?.graphics ?? "auto",
  };

  const consoleLines = useMemo(() => {
    const history = logTail.data ?? [];
    // Live lines that aren't already in the history tail.
    const live = job.state.log.filter((l) => !history.includes(l));
    return [...history, ...live];
  }, [logTail.data, job.state.log]);

  if (!d) {
    return (
      <Screen title="Emulator" actions={detail.error ? <BackLink /> : undefined}>
        <Placeholder>
          {detail.error ? `Could not load this emulator: ${detail.error.ipc.message}` : "Loading…"}
        </Placeholder>
      </Screen>
    );
  }

  const isRunningOrBooting = d.state === "running" || d.state === "booting";
  const hostBlocked = host.data?.verdict === "cannotRun";
  const busy =
    rename.isPending ||
    editHardware.isPending ||
    del.isPending ||
    wipe.isPending ||
    job.state.running;

  return (
    <Screen title={d.displayName} actions={<BackLink />}>
      {/* Identity + rename */}
      <section className="flex flex-wrap items-end gap-2">
        <label className="flex flex-col gap-1 text-[12.5px] text-muted">
          Name
          <input
            type="text"
            data-testid="name-input"
            value={effectiveName}
            onChange={(e) => {
              setNameDraft(e.target.value);
            }}
            className="w-64 rounded-md border border-border-default bg-surface px-3 py-2 text-[13px] text-ink"
          />
        </label>
        <button
          type="button"
          data-testid="rename-button"
          disabled={busy || effectiveName.trim() === "" || effectiveName === d.displayName}
          onClick={() => {
            rename.mutate(
              { id, displayName: effectiveName.trim() },
              {
                onSuccess: () => {
                  setNameDraft(null);
                },
              },
            );
          }}
          className="rounded-md border border-border-default px-3 py-2 text-[12px] disabled:opacity-50"
        >
          Rename
        </button>
        <span
          data-testid="detail-state"
          className={`ml-auto flex items-center gap-1.5 text-[12px] ${d.state === "running" ? "text-running" : d.state === "booting" ? "text-attention" : d.state === "error" ? "text-danger" : "text-muted"}`}
        >
          {d.state === "booting" ? (
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-attention" aria-hidden />
          ) : null}
          {d.state === "booting" ? "booting…" : d.state}
          {d.adbSerial ? ` · ${d.adbSerial}` : ""}
        </span>
      </section>

      {/* Config */}
      <dl className="grid max-w-xl grid-cols-[10rem_1fr] gap-x-4 gap-y-2 rounded-card border border-border-default bg-surface p-4 text-[13px]">
        <Field label="AVD name" value={d.avdName} />
        <Field label="Device profile" value={d.deviceProfileId || "—"} />
        <Field
          label="System image"
          value={
            d.imageCoord
              ? `${d.imageCoord}${d.api === null ? "" : ` · API ${String(d.api)}`}${d.hasPlayStore ? " · Play Store" : ""}`
              : "—"
          }
        />
        <Field label="Source" value={d.source} />
        <Field label="gRPC port" value={"—"} />
        <Field label="Created" value={d.createdAt.slice(0, 19).replace("T", " ")} />
        <Field label="Updated" value={d.updatedAt.slice(0, 19).replace("T", " ")} />
        <Field
          label="Folder"
          value={
            d.avdPath ? (
              <button
                type="button"
                data-testid="open-folder"
                onClick={() => {
                  void revealPath(d.avdPath ?? "");
                }}
                className="text-primary underline underline-offset-2"
              >
                Open folder
              </button>
            ) : (
              "—"
            )
          }
        />
      </dl>

      {/* Hardware */}
      <section className="flex max-w-xl flex-col gap-3 rounded-card border border-border-default bg-surface p-4">
        <h2 className="text-[13px] font-semibold text-ink">Hardware</h2>
        <p className="text-[11.5px] text-muted">
          Saved now; applied the next time this AVD is recreated.
        </p>
        <div className="flex flex-wrap gap-3">
          <label className="flex flex-col gap-1 text-[12.5px] text-muted">
            RAM (MB)
            <input
              type="number"
              data-testid="ram-input"
              value={effectiveHw.ramMb}
              onChange={(e) => {
                setHw({ ...effectiveHw, ramMb: Number(e.target.value) });
              }}
              className="w-28 rounded-md border border-border-default bg-surface px-3 py-2 text-[13px] text-ink"
            />
          </label>
          <label className="flex flex-col gap-1 text-[12.5px] text-muted">
            Storage (MB)
            <input
              type="number"
              data-testid="storage-input"
              value={effectiveHw.storageMb}
              onChange={(e) => {
                setHw({ ...effectiveHw, storageMb: Number(e.target.value) });
              }}
              className="w-28 rounded-md border border-border-default bg-surface px-3 py-2 text-[13px] text-ink"
            />
          </label>
          <label className="flex flex-col gap-1 text-[12.5px] text-muted">
            Graphics
            <select
              data-testid="graphics-select"
              value={effectiveHw.graphics}
              onChange={(e) => {
                setHw({ ...effectiveHw, graphics: e.target.value });
              }}
              className="rounded-md border border-border-default bg-surface px-3 py-2 text-[13px] text-ink"
            >
              {GRAPHICS_OPTIONS.map((g) => (
                <option key={g} value={g}>
                  {g}
                </option>
              ))}
            </select>
          </label>
        </div>
        <button
          type="button"
          data-testid="save-hardware"
          disabled={busy}
          onClick={() => {
            editHardware.mutate(
              { id, ...effectiveHw },
              {
                onSuccess: () => {
                  setHw(null);
                },
              },
            );
          }}
          className="w-fit rounded-md border border-border-default px-3.5 py-2 text-[12px] disabled:opacity-50"
        >
          Save hardware
        </button>
      </section>

      {/* Actions */}
      <section className="flex flex-wrap items-center gap-2">
        {isRunningOrBooting ? (
          <button
            type="button"
            data-testid="stop-button"
            disabled={stop.isPending}
            onClick={() => {
              stop.mutate(id);
            }}
            className="rounded-md border border-border-default px-3.5 py-2 text-[13px] disabled:opacity-50"
          >
            Stop
          </button>
        ) : (
          <button
            type="button"
            data-testid="launch-button"
            disabled={launch.isPending || hostBlocked}
            title={hostBlocked ? host.data?.verdictReason : undefined}
            onClick={() => {
              launch.mutate(id);
            }}
            className="rounded-md bg-primary px-3.5 py-2 text-[13px] font-medium text-white disabled:opacity-50"
          >
            Launch
          </button>
        )}
        {hostBlocked && !isRunningOrBooting ? (
          <span data-testid="launch-blocked" className="text-[12px] text-danger">
            {host.data?.verdictReason}
          </span>
        ) : null}
        <button
          type="button"
          data-testid="wipe-button"
          disabled={busy || isRunningOrBooting}
          onClick={() => {
            wipe.mutate(id);
          }}
          className="rounded-md border border-attention px-3.5 py-2 text-[13px] text-attention disabled:opacity-50"
        >
          Wipe data
        </button>
        {confirmDelete ? (
          <span className="flex items-center gap-2 text-[12.5px]">
            <label className="flex items-center gap-1">
              <input
                type="checkbox"
                data-testid="remove-avd-checkbox"
                checked={alsoRemoveAvd}
                onChange={(e) => {
                  setAlsoRemoveAvd(e.target.checked);
                }}
              />
              also remove the AVD
            </label>
            <button
              type="button"
              data-testid="confirm-delete"
              disabled={busy}
              onClick={() => {
                del.mutate({ id, wipe: alsoRemoveAvd });
              }}
              className="rounded-md bg-danger px-3 py-1.5 text-[12px] font-medium text-white disabled:opacity-50"
            >
              Confirm delete
            </button>
            <button
              type="button"
              onClick={() => {
                setConfirmDelete(false);
              }}
              className="text-muted underline"
            >
              cancel
            </button>
          </span>
        ) : (
          <button
            type="button"
            data-testid="delete-button"
            disabled={busy || isRunningOrBooting}
            onClick={() => {
              setConfirmDelete(true);
            }}
            className="rounded-md border border-danger px-3.5 py-2 text-[13px] text-danger disabled:opacity-50"
          >
            Delete
          </button>
        )}
        {del.isSuccess ? (
          <span data-testid="delete-done" className="text-[12.5px] text-running">
            Deleted —{" "}
            <Link to="/" className="underline">
              back to the dashboard
            </Link>
            .
          </span>
        ) : null}
        <button
          type="button"
          data-testid="export-profile"
          disabled={exportProfile.isPending}
          onClick={() => {
            exportProfile.mutate(id, {
              onSuccess: (json) => {
                // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
                void navigator.clipboard?.writeText(json);
              },
            });
          }}
          className="rounded-md border border-border-default px-3.5 py-2 text-[13px] disabled:opacity-50"
        >
          Export profile
        </button>
      </section>

      {exportProfile.data ? (
        <section className="flex flex-col gap-1">
          <span className="text-[12px] text-muted">
            Copied to the clipboard. Save it as <code>{d.avdName}.emuprofile</code>:
          </span>
          <textarea
            data-testid="exported-profile"
            readOnly
            value={exportProfile.data}
            rows={12}
            className="w-full rounded-md border border-border-default bg-panel p-2 font-mono text-[11px] text-muted"
          />
        </section>
      ) : null}

      {(rename.error ?? editHardware.error ?? wipe.error ?? del.error ?? exportProfile.error) ? (
        <p data-testid="action-error" className="text-[12.5px] text-danger">
          {
            (rename.error ?? editHardware.error ?? wipe.error ?? del.error ?? exportProfile.error)
              ?.ipc.message
          }
        </p>
      ) : null}

      {/* Log console */}
      <section className="flex flex-col gap-2">
        <div className="flex items-center gap-2">
          <h2 className="text-[13px] font-semibold text-ink">Log</h2>
          <button
            type="button"
            data-testid="copy-log"
            onClick={() => {
              // `navigator.clipboard` is absent under jsdom / insecure contexts despite the DOM
              // lib typing it as always present.
              // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
              void navigator.clipboard?.writeText(consoleLines.join("\n"));
            }}
            className="text-[11.5px] text-primary underline"
          >
            Copy
          </button>
          {d.avdPath ? (
            <button
              type="button"
              data-testid="open-log-folder"
              onClick={() => {
                void revealPath(d.avdPath ?? "");
              }}
              className="text-[11.5px] text-primary underline"
            >
              Open AVD folder
            </button>
          ) : null}
        </div>
        <div
          data-testid="log-console"
          className="max-h-72 overflow-y-auto rounded-md bg-panel p-2 font-mono text-[11px] text-muted"
        >
          {consoleLines.length === 0 ? (
            <p>No log yet — launch the emulator to see output here.</p>
          ) : (
            consoleLines.map((line, i) => <p key={`${String(i)}-${line}`}>{line}</p>)
          )}
        </div>
      </section>
    </Screen>
  );
}

function BackLink(): React.JSX.Element {
  return (
    <Link
      to="/"
      data-testid="back-link"
      className="rounded-md border border-border-default px-3 py-2 text-[13px]"
    >
      ← Dashboard
    </Link>
  );
}
