import { Outlet } from "react-router-dom";
import { AppShell } from "./components/AppShell";

/** Root layout: persistent shell wrapping the active route. */
export function App(): React.JSX.Element {
  return (
    <AppShell>
      <Outlet />
    </AppShell>
  );
}
