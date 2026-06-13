import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Configurazione Vite pensata per Tauri:
// - porta fissa 1420 (la stessa indicata in tauri.conf.json)
// - non pulisce lo schermo, così i log di Tauri restano visibili
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "localhost",
  },
});
