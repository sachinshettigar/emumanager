import { Screen, Placeholder } from "../components/Screen";

const COMPONENTS = ["cmdline-tools", "platform-tools (adb)", "emulator", "system-images"] as const;

export function Dependencies(): React.JSX.Element {
  return (
    <Screen title="Dependencies & SDK">
      <ul className="grid gap-2 sm:grid-cols-2">
        {COMPONENTS.map((name) => (
          <li
            key={name}
            className="flex items-center justify-between rounded-card border border-border-default bg-surface px-3.5 py-3 text-[13px]"
          >
            <span>{name}</span>
            <span className="text-[12px] text-attention">not installed</span>
          </li>
        ))}
      </ul>

      <Placeholder>
        Milestone M1 turns this into a real installer: resolve versions from Google&apos;s
        repository, download into the app data directory, and run <code>sdkmanager</code> through
        the managed install — no Android Studio, no terminal.
      </Placeholder>
    </Screen>
  );
}
