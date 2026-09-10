import { Placeholder, Screen } from "../components/Screen";
import { useAppInfo } from "../lib/ipc";

export function About(): React.JSX.Element {
  const info = useAppInfo();

  const d = info.data;
  if (!d) {
    return (
      <Screen title="About">
        <Placeholder>
          {info.error ? `Could not load app info: ${info.error.ipc.message}` : "Loading…"}
        </Placeholder>
      </Screen>
    );
  }

  return (
    <Screen title="About">
      <section className="flex flex-col gap-1 rounded-card border border-border-default bg-surface p-4">
        <h2 className="text-base font-semibold text-ink">{d.name}</h2>
        <p className="text-[13px] text-muted">{d.description}</p>
        <dl className="mt-2 grid grid-cols-[6rem_1fr] gap-x-4 gap-y-1 text-[12.5px]">
          <dt className="text-muted">Version</dt>
          <dd data-testid="about-version" className="tabular-nums">
            {d.version}
          </dd>
          <dt className="text-muted">License</dt>
          <dd data-testid="about-license">{d.license}</dd>
          <dt className="text-muted">Source</dt>
          <dd>
            <a
              href={d.repository}
              target="_blank"
              rel="noreferrer"
              className="text-primary underline underline-offset-2"
            >
              {d.repository.replace(/^https?:\/\//, "")}
            </a>
          </dd>
        </dl>
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="text-[13px] font-semibold text-ink">What it does</h2>
        <ul data-testid="about-features" className="flex flex-col gap-1.5 text-[12.5px] text-muted">
          {d.features.map((f) => (
            <li key={f} className="flex gap-2">
              <span className="text-primary">•</span>
              <span>{f}</span>
            </li>
          ))}
        </ul>
      </section>
    </Screen>
  );
}
