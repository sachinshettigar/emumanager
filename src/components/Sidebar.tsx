import { NavLink } from "react-router-dom";
import { NAV_ITEMS } from "../nav";
import { NavIcon } from "./icons";

const linkBase =
  "mx-2.5 flex items-center gap-2.5 rounded-md px-2.5 py-2 text-[13px] font-medium transition-colors";

/** Left navigation rail. Highlights the active route via NavLink. */
export function Sidebar(): React.JSX.Element {
  return (
    <aside className="flex w-[var(--em-sidebar-w)] shrink-0 flex-col gap-0.5 border-r border-border-default bg-panel py-4">
      <div className="flex items-center gap-2.5 px-4 pb-4 pt-0.5">
        <span
          className="flex h-[26px] w-[26px] items-center justify-center rounded-[7px] border-[1.5px] border-ink font-mono text-[13px] font-semibold"
          aria-hidden="true"
        >
          E
        </span>
        <span className="text-sm font-semibold tracking-tight">Emulator Studio</span>
      </div>

      <nav className="flex flex-col gap-0.5" aria-label="Primary">
        {NAV_ITEMS.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === "/"}
            className={({ isActive }) =>
              isActive
                ? `${linkBase} border border-border-default bg-surface text-ink`
                : `${linkBase} border border-transparent text-muted hover:bg-hover hover:text-ink`
            }
          >
            <NavIcon name={item.icon} />
            {item.label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
