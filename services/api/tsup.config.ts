import { defineConfig } from "tsup";

// @nocapsnap/shared ships TypeScript source, so bundle it into the output
// instead of leaving a require() that plain Node cannot load.
export default defineConfig({
  entry: ["src/server.ts"],
  format: ["cjs"],
  platform: "node",
  target: "node20",
  clean: true,
  noExternal: [/^@nocapsnap\//],
});
