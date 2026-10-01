import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
export default {
  preprocess: vitePreprocess(),
  kit: {
    // Static SPA served by Tauri from inside the app (Spec §2). Routes are
    // prerendered, so the fallback only matters for unknown paths.
    adapter: adapter({ fallback: "200.html" }),
  },
};
