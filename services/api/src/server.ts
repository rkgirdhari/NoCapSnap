import { app } from "./app";

const port = Number(process.env.PORT ?? 3000);

app.listen(port, () => {
  console.log(`[CapSnap API] Hammurabi Coding Company LLC`);
  console.log(`[CapSnap API] listening on :${port}`);
});
