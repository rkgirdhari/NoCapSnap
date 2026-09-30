import { app } from "./app";
import { config } from "./config";

app.listen(config.port, () => {
  console.log(`[CapSnap API] Hammurabi Coding Company LLC`);
  console.log(`[CapSnap API] listening on :${config.port}`);
  console.log(`[CapSnap API] storage: ${config.storage.driver}`);
});
