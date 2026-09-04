import { Screen, Placeholder } from "../components/Screen";

export function Dashboard(): React.JSX.Element {
  return (
    <Screen
      title="My emulators"
      actions={
        <span className="rounded-md bg-primary px-3.5 py-2 text-[13px] font-medium text-white">
          Create emulator
        </span>
      }
    >
      <div className="flex flex-wrap items-center gap-3.5 rounded-card border border-border-default bg-surface px-3.5 py-3 text-[12.5px] text-muted">
        <span className="font-semibold text-ink">Host</span>
        <span>Not detected yet</span>
        <span className="text-faint">|</span>
        <span>Accelerator: unknown</span>
        <span className="text-faint">|</span>
        <span>Disk free: —</span>
      </div>

      <Placeholder>
        No emulators yet. Once the SDK is installed on the Dependencies screen, created emulators
        will be listed here with live Stopped / Booting / Running state.
      </Placeholder>
    </Screen>
  );
}
