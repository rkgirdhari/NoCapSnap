# W2 screenshot and check harness

Test tooling only. Nothing here ships in the app.

Setup, in a scratch directory outside the repo:
`npm i playwright sharp jsqr`, plus Chromium (`/opt/pw-browsers` in the build sandbox).

| Script | What it does |
|---|---|
| `w2fix.mjs` | Crops dish photos out of the owner's mockups to use as camera input |
| `w2shots.mjs out fix y4m fonts` | Drives the browser preview build (`VITE_CAPSNAP_PREVIEW=1 pnpm build && pnpm preview`) through the whole flow at 375×667 @3x, America/Chicago, fixed Tuesday-evening clock. It uses Roboto and Noto Serif, which Android's WebView uses for system-ui and serif, loaded from `@fontsource-variable/*` (OFL-1.1). |
| `w2side.mjs` / `w2strip.mjs` | Builds the mockup-vs-build images in `..` |
| `qrcheck.mjs files…` | Runs jsQR at three scales. A real QR is the positive control, and the concept QR must not decode. |
| `b64check.mjs` | Checks that the Android upload encoder (FileReader base64) matches Node's base64 byte for byte |

The scratch paths were replaced with `$SCRATCH` and `$REPO`. Edit them before running.
