import { useMemo, useState } from "react";

import { Placeholder, Screen } from "../components/Screen";
import type { DeviceInfo, ImageInfo } from "../lib/bindings";
import {
  useCreateEmulator,
  useDevices,
  useEmulatorJob,
  useImages,
  useSaveProfile,
  type EmulatorJobState,
} from "../lib/ipc";

const STEPS = ["Device", "System image", "Hardware", "Review"] as const;

function formatSize(bytes: number): string {
  if (bytes <= 0) {
    return "size unknown";
  }
  const mb = bytes / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb).toString()} MB`;
}

function StepTabs({ current }: { current: number }): React.JSX.Element {
  return (
    <ol className="flex flex-wrap gap-2 text-[12.5px] text-muted">
      {STEPS.map((label, i) => (
        <li
          key={label}
          data-active={i === current}
          className="rounded-md border border-border-default bg-surface px-3 py-1.5 data-[active=true]:border-primary data-[active=true]:text-ink"
        >
          <span className="font-mono text-faint">{i + 1}</span> {label}
        </li>
      ))}
    </ol>
  );
}

/** Device form factors in the order the grouped list shows them. Anything unknown → "Other". */
const FORM_FACTOR_ORDER = [
  "phone",
  "tablet",
  "foldable",
  "wear",
  "tv",
  "automotive",
  "desktop",
] as const;

const FORM_FACTOR_LABEL: Record<string, string> = {
  phone: "Phones",
  tablet: "Tablets",
  foldable: "Foldables",
  wear: "Wear OS",
  tv: "Android TV",
  automotive: "Automotive",
  desktop: "Desktop",
  other: "Other",
};

function DeviceRow({
  device,
  selectedId,
  onSelect,
}: {
  device: DeviceInfo;
  selectedId: string | null;
  onSelect: (id: string) => void;
}): React.JSX.Element {
  return (
    <button
      type="button"
      data-testid={`device-${device.id}`}
      data-selected={device.id === selectedId}
      onClick={() => {
        onSelect(device.id);
      }}
      className="flex w-full flex-col gap-0.5 rounded-card border border-border-default bg-surface px-3.5 py-3 text-left text-[13px] data-[selected=true]:border-primary"
    >
      <span className="flex items-center gap-2">
        {device.name}
        {device.skin ? (
          <span className="rounded bg-running/15 px-1.5 py-0.5 text-[10px] text-running">
            frame
          </span>
        ) : null}
      </span>
      <span className="text-[11px] text-muted">
        {device.oem ? `${device.oem} · ` : ""}
        {device.resolution} · {device.densityDpi}dpi · {device.ramMb} MB RAM
      </span>
    </button>
  );
}

function DeviceStep({
  devices,
  selectedId,
  onSelect,
}: {
  devices: DeviceInfo[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}): React.JSX.Element {
  const [query, setQuery] = useState("");
  const q = query.trim().toLowerCase();
  // Every group is expanded by default (like Android Studio's device manager); collapsing is
  // opt-in and remembered. A search always forces every matching group open.
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set());

  const groups = useMemo(() => {
    const filtered = q
      ? devices.filter((d) => `${d.name} ${d.oem} ${d.id}`.toLowerCase().includes(q))
      : devices;
    const byFactor = new Map<string, DeviceInfo[]>();
    for (const d of filtered) {
      const key = (FORM_FACTOR_ORDER as readonly string[]).includes(d.formFactor)
        ? d.formFactor
        : "other";
      const list = byFactor.get(key) ?? [];
      list.push(d);
      byFactor.set(key, list);
    }
    const keys = [...FORM_FACTOR_ORDER, "other"].filter((k) => byFactor.has(k));
    return keys.map((k) => ({
      key: k,
      label: FORM_FACTOR_LABEL[k] ?? k,
      items: byFactor.get(k) ?? [],
    }));
  }, [devices, q]);

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      <input
        type="search"
        data-testid="device-search"
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
        }}
        placeholder="Search devices…"
        className="rounded-md border border-border-default bg-surface px-3 py-2 text-[13px]"
      />
      <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto">
        {groups.length === 0 ? (
          <Placeholder>No devices match “{query}”.</Placeholder>
        ) : (
          groups.map((group) => (
            <details
              key={group.key}
              data-testid={`device-group-${group.key}`}
              open={q !== "" || !collapsed.has(group.key)}
              onToggle={(e) => {
                const isOpen = e.currentTarget.open;
                setCollapsed((prev) => {
                  const next = new Set(prev);
                  if (isOpen) {
                    next.delete(group.key);
                  } else {
                    next.add(group.key);
                  }
                  return next;
                });
              }}
              className="rounded-card border border-border-default bg-panel/40"
            >
              <summary className="cursor-pointer px-3 py-2 text-[12px] font-semibold text-ink">
                {group.label} <span className="font-normal text-muted">({group.items.length})</span>
              </summary>
              <ul className="grid gap-2 p-2 sm:grid-cols-2">
                {group.items.map((device) => (
                  <li key={device.id}>
                    <DeviceRow device={device} selectedId={selectedId} onSelect={onSelect} />
                  </li>
                ))}
              </ul>
            </details>
          ))
        )}
      </div>
    </div>
  );
}

function ImageStep({
  images,
  selectedCoord,
  onSelect,
}: {
  images: ImageInfo[];
  selectedCoord: string | null;
  onSelect: (coord: string) => void;
}): React.JSX.Element {
  return (
    <ul className="grid min-h-0 flex-1 auto-rows-min gap-2 overflow-y-auto pr-1 sm:grid-cols-2">
      {images.map((image) => (
        <li key={image.coord}>
          <button
            type="button"
            data-testid={`image-${image.coord}`}
            data-selected={image.coord === selectedCoord}
            onClick={() => {
              onSelect(image.coord);
            }}
            className="flex h-full w-full flex-col gap-0.5 rounded-card border border-border-default bg-surface px-3.5 py-3 text-left text-[13px] data-[selected=true]:border-primary"
          >
            <span>
              Android {image.androidVersion} · API {image.api}
              {image.hasPlayStore ? " · Play Store" : ""}
            </span>
            <span className="text-[11px] text-muted">
              {image.imageType} · {image.abi} ·{" "}
              {image.installed ? "installed" : formatSize(image.downloadSizeBytes)}
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}

function NumberField({
  label,
  value,
  onChange,
  testId,
}: {
  label: string;
  value: number;
  onChange: (n: number) => void;
  testId: string;
}): React.JSX.Element {
  return (
    <label className="flex flex-col gap-1 text-[12.5px] text-muted">
      {label}
      <input
        type="number"
        data-testid={testId}
        value={value}
        onChange={(e) => {
          onChange(Number(e.target.value));
        }}
        className="rounded-md border border-border-default bg-surface px-3 py-2 text-[13px] text-ink"
      />
    </label>
  );
}

function JobPanel({ run }: { run: EmulatorJobState }): React.JSX.Element | null {
  if (!run.running && run.log.length === 0 && !run.error) {
    return null;
  }
  let heading: string;
  if (run.running) {
    heading = run.phase ?? "Working…";
  } else if (run.error) {
    heading = "Failed";
  } else {
    heading = "Done";
  }
  return (
    <div className="rounded-card border border-border-default bg-surface p-3.5 text-[12.5px]">
      <div className="mb-2 flex items-center justify-between">
        <span className="font-semibold text-ink">{heading}</span>
        {run.pct !== null ? <span className="text-muted">{run.pct}%</span> : null}
      </div>
      {run.error ? <p className="mb-2 text-danger">{run.error}</p> : null}
      <div
        data-testid="create-log"
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

export function Create(): React.JSX.Element {
  const devicesQuery = useDevices();
  const imagesQuery = useImages();
  const create = useCreateEmulator();
  const saveProfile = useSaveProfile();
  const { state: run, reset } = useEmulatorJob();

  const [step, setStep] = useState(0);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [imageCoord, setImageCoord] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [ramMb, setRamMb] = useState(2048);
  const [storageMb, setStorageMb] = useState(6144);
  const [deviceFrame, setDeviceFrame] = useState(true);

  const device = devicesQuery.data?.find((d) => d.id === deviceId) ?? null;
  const image = imagesQuery.data?.find((i) => i.coord === imageCoord) ?? null;
  const effectiveName =
    name.trim() || (device && image ? `${device.name} · API ${image.api.toString()}` : "");

  const busy = create.isPending || run.running;

  const submit = (launch: boolean): void => {
    if (!deviceId || !imageCoord) {
      return;
    }
    reset();
    create.mutate({
      name: effectiveName,
      deviceId,
      imageCoord,
      ramMb,
      storageMb,
      deviceFrame,
      launch,
    });
  };

  const saveAsProfile = (): void => {
    if (!deviceId || !imageCoord || !effectiveName) {
      return;
    }
    // imageCoord is `system-images;android-NN;<type>;<abi>`.
    const parts = imageCoord.split(";");
    const rawType = parts[2] ?? "default";
    const profile = {
      schemaVersion: "1.0",
      name: effectiveName,
      platform: "android",
      device: { profile: deviceId },
      image: {
        api: Number((parts[1] ?? "android-0").replace("android-", "")),
        type: rawType === "android-automotive-playstore" ? "android-automotive_playstore" : rawType,
        abi: parts[3] ?? "x86_64",
      },
      hardware: { ramMb, storageGb: Math.max(2, Math.ceil(storageMb / 1024)) },
    };
    saveProfile.mutate(Array.from(new TextEncoder().encode(JSON.stringify(profile))));
  };

  const canAdvance =
    (step === 0 && deviceId !== null) || (step === 1 && imageCoord !== null) || step === 2;

  let body: React.JSX.Element;
  if (step === 0) {
    if (devicesQuery.isPending) {
      body = <Placeholder>Loading device profiles…</Placeholder>;
    } else if (devicesQuery.error) {
      body = <Placeholder>Could not load devices: {devicesQuery.error.ipc.message}</Placeholder>;
    } else {
      body = (
        <DeviceStep devices={devicesQuery.data} selectedId={deviceId} onSelect={setDeviceId} />
      );
    }
  } else if (step === 1) {
    if (imagesQuery.isPending) {
      body = <Placeholder>Loading system images…</Placeholder>;
    } else if (imagesQuery.error) {
      body = <Placeholder>Could not load images: {imagesQuery.error.ipc.message}</Placeholder>;
    } else {
      body = (
        <ImageStep images={imagesQuery.data} selectedCoord={imageCoord} onSelect={setImageCoord} />
      );
    }
  } else if (step === 2) {
    body = (
      <div className="flex max-w-md flex-col gap-3">
        <label className="flex flex-col gap-1 text-[12.5px] text-muted">
          Name
          <input
            type="text"
            data-testid="name-input"
            value={name}
            onChange={(e) => {
              setName(e.target.value);
            }}
            placeholder={effectiveName}
            className="rounded-md border border-border-default bg-surface px-3 py-2 text-[13px] text-ink"
          />
        </label>
        <NumberField label="RAM (MB)" value={ramMb} onChange={setRamMb} testId="ram-input" />
        <NumberField
          label="Internal storage (MB)"
          value={storageMb}
          onChange={setStorageMb}
          testId="storage-input"
        />
        <label className="flex items-center gap-2 text-[13px] text-ink">
          <input
            type="checkbox"
            data-testid="device-frame-input"
            checked={deviceFrame}
            onChange={(e) => {
              setDeviceFrame(e.target.checked);
            }}
          />
          Show device frame (bezel)
          {device && device.skin === null ? (
            <span className="text-[11px] text-muted">— this device has no dedicated frame</span>
          ) : null}
        </label>
      </div>
    );
  } else {
    body = (
      <dl
        data-testid="review"
        className="grid max-w-md grid-cols-[8rem_1fr] gap-x-4 gap-y-2 rounded-card border border-border-default bg-surface p-4 text-[13px]"
      >
        <dt className="text-muted">Name</dt>
        <dd>{effectiveName}</dd>
        <dt className="text-muted">Device</dt>
        <dd>{device?.name ?? "—"}</dd>
        <dt className="text-muted">Image</dt>
        <dd>{image ? `${image.imageType} · API ${image.api.toString()} · ${image.abi}` : "—"}</dd>
        <dt className="text-muted">RAM</dt>
        <dd>{ramMb} MB</dd>
        <dt className="text-muted">Storage</dt>
        <dd>{storageMb} MB</dd>
        <dt className="text-muted">Device frame</dt>
        <dd>{deviceFrame ? "on" : "off"}</dd>
      </dl>
    );
  }

  const actions =
    step < 3 ? (
      <div className="flex items-center gap-2">
        {step > 0 ? (
          <button
            type="button"
            data-testid="back-button"
            onClick={() => {
              setStep((s) => s - 1);
            }}
            className="rounded-md border border-border-default px-3.5 py-2 text-[13px]"
          >
            Back
          </button>
        ) : null}
        <button
          type="button"
          data-testid="next-button"
          disabled={!canAdvance}
          onClick={() => {
            setStep((s) => s + 1);
          }}
          className="rounded-md bg-primary px-3.5 py-2 text-[13px] font-medium text-white disabled:opacity-50"
        >
          Next
        </button>
      </div>
    ) : (
      <div className="flex items-center gap-2">
        <button
          type="button"
          data-testid="back-button"
          onClick={() => {
            setStep((s) => s - 1);
          }}
          className="rounded-md border border-border-default px-3.5 py-2 text-[13px]"
        >
          Back
        </button>
        <button
          type="button"
          data-testid="save-as-profile"
          disabled={saveProfile.isPending}
          onClick={saveAsProfile}
          className="rounded-md border border-border-default px-3.5 py-2 text-[13px] disabled:opacity-50"
        >
          {saveProfile.isSuccess ? "Saved" : "Save as profile"}
        </button>
        <button
          type="button"
          data-testid="create-button"
          disabled={busy}
          onClick={() => {
            submit(false);
          }}
          className="rounded-md border border-primary px-3.5 py-2 text-[13px] font-medium text-primary disabled:opacity-50"
        >
          Create
        </button>
        <button
          type="button"
          data-testid="create-launch-button"
          disabled={busy}
          onClick={() => {
            submit(true);
          }}
          className="rounded-md bg-primary px-3.5 py-2 text-[13px] font-medium text-white disabled:opacity-50"
        >
          {busy ? "Working…" : "Create & launch"}
        </button>
      </div>
    );

  const done = !run.running && !run.error && create.isSuccess;

  return (
    <Screen title="Create emulator" actions={actions}>
      <StepTabs current={step} />
      {done ? (
        <div
          data-testid="create-success"
          className="rounded-card border border-running bg-surface p-4 text-[13px] text-running"
        >
          Emulator created. It&apos;s on the dashboard now.
        </div>
      ) : (
        body
      )}
      <JobPanel run={run} />
      {create.error && !run.error ? (
        <p className="text-[12.5px] text-danger">{create.error.ipc.message}</p>
      ) : null}
    </Screen>
  );
}
