import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { readFileSync } from "node:fs";

const versione = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")).version;

// Configurazione Vite pensata per Tauri:
// - porta fissa 1420 (la stessa indicata in tauri.conf.json)
// - non pulisce lo schermo, così i log di Tauri restano visibili
// - la versione mostrata nell'app viene da package.json: scritta a mano restava
//   indietro (la 1.3.0 installata si presentava come "v1.2.0")
// La schermata di avvio in index.html compare prima del bundle, quindi la
// versione non può arrivarle da JS: la si scrive nell'HTML al momento del build.
const versioneInHtml = {
  name: "versione-in-html",
  transformIndexHtml: (html) => html.replaceAll("__VERSIONE__", versione),
};

export default defineConfig({
  define: { __VERSIONE__: JSON.stringify(versione) },
  plugins: [svelte(), versioneInHtml],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "localhost",
  },
});
