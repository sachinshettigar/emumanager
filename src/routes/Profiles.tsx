import { useState } from "react";
import { Link } from "react-router-dom";

import { Placeholder, Screen } from "../components/Screen";
import type { ProfileInspection } from "../lib/bindings";
import {
  getSavedProfile,
  useApplyProfile,
  useDeleteProfile,
  useEmulatorJob,
  useInspectProfile,
  useProfiles,
  useSaveProfile,
} from "../lib/ipc";

function formatBytes(n: number): string {
  if (n <= 0) return "—";
  const mb = n / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb).toString()} MB`;
}

async function readBytes(file: File): Promise<number[]> {
  const buf = await new Promise<ArrayBuffer>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      resolve(reader.result as ArrayBuffer);
    };
    reader.onerror = () => {
      reject(reader.error ?? new Error("could not read the file"));
    };
    reader.readAsArrayBuffer(file);
  });
  return Array.from(new Uint8Array(buf));
}

/** The parsed-and-resolved preview of a dropped file, plus its raw bytes for Apply. */
interface Loaded {
  bytes: number[];
  inspection: ProfileInspection;
}

export function Profiles(): React.JSX.Element {
  const inspect = useInspectProfile();
  const apply = useApplyProfile();
  const save = useSaveProfile();
  const del = useDeleteProfile();
  const saved = useProfiles();
  const { state: job, reset: resetJob } = useEmulatorJob();

  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [rejection, setRejection] = useState<string | null>(null);

  const handleFile = (file: File | undefined): void => {
    if (!file) return;
    setRejection(null);
    setLoaded(null);
    void readBytes(file).then((bytes) => {
      inspect.mutate(bytes, {
        onSuccess: (inspection) => {
          setLoaded({ bytes, inspection });
        },
        onError: (e) => {
          setRejection(e.ipc.message);
        },
      });
    });
  };

  const runApply = (launch: boolean): void => {
    if (!loaded) return;
    resetJob();
    apply.mutate({ bytes: loaded.bytes, launch });
  };

  const applySaved = (name: string): void => {
    setRejection(null);
    void getSavedProfile(name).then(
      (json) => {
        const bytes = Array.from(new TextEncoder().encode(json));
        inspect.mutate(bytes, {
          onSuccess: (inspection) => {
            setLoaded({ bytes, inspection });
          },
          onError: (e) => {
            setRejection(e.ipc.message);
          },
        });
      },
      (e: unknown) => {
        setRejection(e instanceof Error ? e.message : "could not load that profile");
      },
    );
  };

  const applyDone = !job.running && !job.error && apply.isSuccess;

  return (
    <Screen title="Profiles">
      <label
        data-testid="drop-zone"
        onDragOver={(e) => {
          e.preventDefault();
          setDragOver(true);
        }}
        onDragLeave={() => {
          setDragOver(false);
        }}
        onDrop={(e) => {
          e.preventDefault();
          setDragOver(false);
          handleFile(e.dataTransfer.files[0]);
        }}
        className={`flex cursor-pointer flex-col items-center gap-1 rounded-card border border-dashed p-8 text-center text-sm ${
          dragOver ? "border-primary bg-panel" : "border-border-default bg-surface text-muted"
        }`}
      >
        Drop a <code>.emuprofile</code> here, or click to choose a file
        <input
          type="file"
          accept=".emuprofile,.json,application/json"
          data-testid="file-input"
          className="hidden"
          onChange={(e) => {
            handleFile(e.target.files?.[0]);
          }}
        />
      </label>

      {rejection ? (
        <p data-testid="rejection" className="text-[12.5px] text-danger">
          {rejection}
        </p>
      ) : null}

      {inspect.isPending ? <Placeholder>Reading the profile…</Placeholder> : null}

      {loaded ? (
        <section
          data-testid="preview"
          className="flex flex-col gap-3 rounded-card border border-border-default bg-surface p-4 text-[13px]"
        >
          <div>
            <h2 className="text-sm font-semibold text-ink">{loaded.inspection.name}</h2>
            {loaded.inspection.description ? (
              <p className="text-[12px] text-muted">{loaded.inspection.description}</p>
            ) : null}
          </div>
          <dl className="grid grid-cols-[7rem_1fr] gap-x-4 gap-y-1 text-[12.5px]">
            <dt className="text-muted">Device</dt>
            <dd>{loaded.inspection.deviceLabel}</dd>
            <dt className="text-muted">Image</dt>
            <dd>{loaded.inspection.imageLabel}</dd>
          </dl>

          <table className="w-full text-left text-[12.5px]">
            <thead className="text-muted">
              <tr>
                <th className="py-1 font-medium">Requirement</th>
                <th className="py-1 font-medium">Status</th>
              </tr>
            </thead>
            <tbody data-testid="requirements">
              {loaded.inspection.requirements.map((r) => (
                <tr key={r.label} className="border-t border-border-default">
                  <td className="py-1">{r.label}</td>
                  <td className="py-1">
                    {r.present ? (
                      <span className="text-running">installed</span>
                    ) : (
                      <span className="text-attention">
                        download {formatBytes(r.downloadBytes)}
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          <p className="text-[12px] text-muted">
            {loaded.inspection.ready
              ? "Everything is already installed — Apply creates the emulator right away."
              : `Total download: ${formatBytes(loaded.inspection.totalDownloadBytes)}`}
          </p>

          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              data-testid="apply-button"
              disabled={apply.isPending || job.running}
              onClick={() => {
                runApply(false);
              }}
              className="rounded-md border border-primary px-3.5 py-2 text-[13px] font-medium text-primary disabled:opacity-50"
            >
              Apply
            </button>
            <button
              type="button"
              data-testid="apply-launch-button"
              disabled={apply.isPending || job.running}
              onClick={() => {
                runApply(true);
              }}
              className="rounded-md bg-primary px-3.5 py-2 text-[13px] font-medium text-white disabled:opacity-50"
            >
              {job.running ? "Working…" : "Apply & launch"}
            </button>
            <button
              type="button"
              data-testid="save-to-library"
              disabled={save.isPending}
              onClick={() => {
                save.mutate(loaded.bytes);
              }}
              className="rounded-md border border-border-default px-3.5 py-2 text-[13px] disabled:opacity-50"
            >
              {save.isSuccess ? "Saved" : "Save to library"}
            </button>
          </div>

          {(job.log.length > 0 || job.error) && (
            <div
              data-testid="apply-log"
              className="max-h-40 overflow-y-auto rounded-md bg-panel p-2 font-mono text-[11px] text-muted"
            >
              {job.error ? <p className="text-danger">{job.error}</p> : null}
              {job.log.map((line, i) => (
                <p key={`${String(i)}-${line}`}>{line}</p>
              ))}
            </div>
          )}

          {applyDone ? (
            <p data-testid="apply-success" className="text-[12.5px] text-running">
              Emulator created —{" "}
              <Link to="/" className="underline">
                see it on the dashboard
              </Link>
              .
            </p>
          ) : null}
          {apply.error && !job.error ? (
            <p className="text-[12.5px] text-danger">{apply.error.ipc.message}</p>
          ) : null}
        </section>
      ) : null}

      <section className="flex flex-col gap-2">
        <h2 className="text-[13px] font-semibold text-ink">Saved profiles</h2>
        {saved.isPending ? (
          <Placeholder>Loading…</Placeholder>
        ) : saved.error ? (
          <Placeholder>Could not load saved profiles: {saved.error.ipc.message}</Placeholder>
        ) : saved.data.length === 0 ? (
          <Placeholder>
            No saved profiles yet. Import one above and choose “Save to library”, or use “Save as
            profile” in the Create wizard.
          </Placeholder>
        ) : (
          <ul className="flex flex-col gap-2">
            {saved.data.map((p) => (
              <li
                key={p.name}
                data-testid={`saved-${p.name}`}
                className="flex items-center justify-between rounded-card border border-border-default bg-surface px-3.5 py-3 text-[13px]"
              >
                <div className="flex flex-col gap-0.5">
                  <span>{p.name}</span>
                  {p.description ? (
                    <span className="text-[11px] text-muted">{p.description}</span>
                  ) : null}
                </div>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    data-testid={`apply-saved-${p.name}`}
                    onClick={() => {
                      applySaved(p.name);
                    }}
                    className="rounded-md border border-border-default px-3 py-1.5 text-[12px]"
                  >
                    Load
                  </button>
                  <button
                    type="button"
                    data-testid={`delete-saved-${p.name}`}
                    disabled={del.isPending}
                    onClick={() => {
                      del.mutate(p.name);
                    }}
                    className="rounded-md border border-danger px-3 py-1.5 text-[12px] text-danger disabled:opacity-50"
                  >
                    Delete
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </Screen>
  );
}
