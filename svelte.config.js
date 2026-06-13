import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// Preprocessore standard di Svelte (gestisce TypeScript, PostCSS, ecc. se servisse).
export default {
  preprocess: vitePreprocess(),
};
