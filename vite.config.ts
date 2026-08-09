import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri serves these as static assets from inside the binary over a custom
// protocol (ADR-003) — there is no server in the shipped application, so the
// build target is a plain static bundle.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
  server: { port: 5173, strictPort: true },
});
