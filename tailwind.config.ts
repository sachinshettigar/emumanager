import type { Config } from "tailwindcss";

// Colors are declared once in src/styles/tokens.css as CSS custom properties and
// surfaced here as Tailwind utilities. Components use the semantic names
// (bg-surface, text-muted, border-default, text-primary...) — never raw hex.
const config: Config = {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        app: "var(--em-bg-app)",
        panel: "var(--em-bg-panel)",
        surface: "var(--em-bg-surface)",
        hover: "var(--em-bg-hover)",
        "border-subtle": "var(--em-border-subtle)",
        "border-default": "var(--em-border-default)",
        ink: "var(--em-text-ink)",
        muted: "var(--em-text-muted)",
        faint: "var(--em-text-faint)",
        primary: "var(--em-primary)",
        "primary-hover": "var(--em-primary-hover)",
        running: "var(--em-running)",
        attention: "var(--em-attention)",
        danger: "var(--em-danger)",
      },
      fontFamily: {
        sans: ["'IBM Plex Sans'", "system-ui", "sans-serif"],
        mono: ["'IBM Plex Mono'", "ui-monospace", "monospace"],
      },
      borderRadius: {
        card: "8px",
      },
    },
  },
  plugins: [],
};

export default config;
