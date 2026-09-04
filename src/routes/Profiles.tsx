import { Screen, Placeholder } from "../components/Screen";

export function Profiles(): React.JSX.Element {
  return (
    <Screen title="Profiles">
      <div className="rounded-card border border-dashed border-border-default bg-surface p-8 text-center text-sm text-muted">
        Drop a <code>.emuprofile</code> here to import
      </div>

      <Placeholder>
        Milestone M4. A profile is a recipe only — it never carries SDK or system-image bytes. On
        import, EmuManager resolves what the recipe needs, shows a requirement diff, downloads
        locally, and recreates the emulator.
      </Placeholder>
    </Screen>
  );
}
