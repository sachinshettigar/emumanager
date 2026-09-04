import type { ReactNode } from "react";
import { Sidebar } from "./Sidebar";

interface AppShellProps {
  children: ReactNode;
}

/** Persistent app chrome: left sidebar + scrollable main content area. */
export function AppShell({ children }: AppShellProps): React.JSX.Element {
  return (
    <div className="flex h-full w-full overflow-hidden bg-app text-ink">
      <Sidebar />
      <main className="flex min-w-0 flex-1 flex-col overflow-y-auto">{children}</main>
    </div>
  );
}
