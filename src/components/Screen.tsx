import type { ReactNode } from "react";

interface ScreenProps {
  title: string;
  /** Optional right-aligned toolbar content (buttons, search). */
  actions?: ReactNode;
  children: ReactNode;
}

/** Standard route frame: sticky title bar + padded content column. */
export function Screen({ title, actions, children }: ScreenProps): React.JSX.Element {
  return (
    <>
      <header className="flex h-[var(--em-topbar-h)] shrink-0 items-center justify-between border-b border-border-default px-6">
        <h1 className="text-base font-semibold tracking-tight">{title}</h1>
        {actions ? <div className="flex items-center gap-2.5">{actions}</div> : null}
      </header>
      <div className="flex min-h-0 flex-1 flex-col gap-4 p-6">{children}</div>
    </>
  );
}

interface PlaceholderProps {
  children: ReactNode;
}

/** Neutral "nothing here yet" panel used by the M0 static screens. */
export function Placeholder({ children }: PlaceholderProps): React.JSX.Element {
  return (
    <div className="rounded-card border border-dashed border-border-default bg-surface p-8 text-sm text-muted">
      {children}
    </div>
  );
}
