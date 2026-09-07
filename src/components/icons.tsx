import type { NavItem } from "../nav";

const shared = {
  width: 18,
  height: 18,
  viewBox: "0 0 20 20",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.6,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

/** One line-icon per nav item, keyed by NavItem["icon"]. */
export function NavIcon({ name }: { name: NavItem["icon"] }): React.JSX.Element {
  switch (name) {
    case "dashboard":
      return (
        <svg {...shared} aria-hidden="true">
          <rect x="3" y="3" width="6" height="6" rx="1" />
          <rect x="11" y="3" width="6" height="6" rx="1" />
          <rect x="3" y="11" width="6" height="6" rx="1" />
          <rect x="11" y="11" width="6" height="6" rx="1" />
        </svg>
      );
    case "create":
      return (
        <svg {...shared} aria-hidden="true">
          <rect x="6" y="3" width="8" height="14" rx="1.5" />
          <line x1="8.5" y1="14.5" x2="11.5" y2="14.5" />
        </svg>
      );
    case "dependencies":
      return (
        <svg {...shared} aria-hidden="true">
          <path d="M10 3v9" />
          <path d="M6.5 9 10 12.5 13.5 9" />
          <path d="M4 16h12" />
        </svg>
      );
    case "profiles":
      return (
        <svg {...shared} aria-hidden="true">
          <path d="M6 3h5l3 3v11H6z" />
          <path d="M11 3v3h3" />
        </svg>
      );
    case "about":
      return (
        <svg {...shared} aria-hidden="true">
          <circle cx="10" cy="10" r="7" />
          <line x1="10" y1="9" x2="10" y2="14" />
          <circle cx="10" cy="6.5" r="0.6" fill="currentColor" />
        </svg>
      );
  }
}
