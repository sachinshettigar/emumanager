import { Screen, Placeholder } from "../components/Screen";

const STEPS = ["Device", "System image", "Hardware", "Review"] as const;

export function Create(): React.JSX.Element {
  return (
    <Screen title="Create emulator">
      <ol className="flex flex-wrap gap-2 text-[12.5px] text-muted">
        {STEPS.map((step, i) => (
          <li key={step} className="rounded-md border border-border-default bg-surface px-3 py-1.5">
            <span className="font-mono text-faint">{i + 1}</span> {step}
          </li>
        ))}
      </ol>

      <Placeholder>
        The create wizard is wired in milestone M2: pick a device profile, choose and download a
        system image, set RAM / storage, then review and launch. Nothing here talks to the Android
        SDK yet.
      </Placeholder>
    </Screen>
  );
}
