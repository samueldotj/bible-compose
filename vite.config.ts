import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri serves these as static assets from inside the binary over a custom
// protocol (ADR-003) — there is no server in the shipped application, so the
// build target is a plain static bundle.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
  server: {
    port: 5173,
    strictPort: true,
    // The Rust half builds into `target/`, inside the same folder Vite
    // watches. Left alone, the watcher opens every file Cargo writes and dies
    // on the first one still held open — `EBUSY: resource busy or locked,
    // watch target/debug/build/…/build_script_build.exe` — which takes the
    // dev server, and therefore `tauri dev`, down with it. Nothing under these
    // two directories can change what the frontend serves anyway.
    watch: { ignored: ["**/target/**", "**/crates/**"] },
  },
});
