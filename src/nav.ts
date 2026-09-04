/** Nav model shared by the router and the sidebar so they can never drift. */
export interface NavItem {
  readonly to: string;
  readonly label: string;
  /** Simple key used to pick the icon in Sidebar. */
  readonly icon: "dashboard" | "create" | "dependencies" | "profiles";
}

export const NAV_ITEMS: readonly NavItem[] = [
  { to: "/", label: "Dashboard", icon: "dashboard" },
  { to: "/create", label: "Create emulator", icon: "create" },
  { to: "/dependencies", label: "Dependencies", icon: "dependencies" },
  { to: "/profiles", label: "Profiles", icon: "profiles" },
];
