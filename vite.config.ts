import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri expects the built frontend at ../dist relative to src-tauri
// (see src-tauri/tauri.conf.json -> build.frontendDist).
export default defineConfig({
  plugins: [react()],
  // Tauri dev server: fixed port, fail instead of falling back, no HMR clobbering.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: true,
  },
});
