# CapSnap (nocapsnap)

Staff-captured dish photos tied to customer reviews.
**Hammurabi Coding Company LLC** · Founder: **R. K. Girdhari**

| | |
|---|---|
| Display name | CapSnap |
| Package / slug | `nocapsnap` |
| Android applicationId | `com.hammurabicoding.nocapsnap` |

## Layout

```
nocapsnap/
├── apps/
│   └── mobile/          # Expo (SDK 57) React Native app → Google Play
├── services/
│   └── api/             # Express + TypeScript API
├── packages/
│   └── shared/          # Shared types + constants (TypeScript source)
├── pnpm-workspace.yaml
├── turbo.json
└── .env.example
```

## First run

Requires Node 20+ and Corepack (ships with Node).

```bash
cd nocapsnap
corepack enable
pnpm install

# API on :3000. API_URL is the base for locally stored file URLs (QR image,
# photos), so use an address the phone can reach — 10.0.2.2 for the emulator.
API_URL=http://10.0.2.2:3000 pnpm dev:api
curl localhost:3000/health

# Mobile — point the app at your API first
cp apps/mobile/.env.example apps/mobile/.env   # 10.0.2.2 = host from the Android emulator
pnpm dev:mobile                                # press "a" for Android
# or build and install a native debug build on an emulator/device:
pnpm android
```

Useful checks:

```bash
pnpm typecheck                        # all packages
pnpm test                             # API unit tests (vitest)
pnpm build                            # bundles the API to services/api/dist
pnpm --filter @nocapsnap/mobile run doctor   # expo-doctor
```

## API

| Method | Path | Status |
|---|---|---|
| GET | `/health` | Live — returns product / company / founder |
| POST | `/api/photos/capture` | Live — stores the photo set and returns a `Photo` (**201**) |
| GET | `/api/photos/qr/:token` | Live — looks the photo up by its QR token (in-memory store) |
| POST | `/api/reviews` | Validates body with zod, then **501** (stub) |

Errors always come back as `{ "error": string, "code"?: string }`
(`ApiError` in `@nocapsnap/shared`): 400 for validation, bad JSON or an unreadable
image, 413 for oversized uploads, 501 for unimplemented services, 500 otherwise.

### Photo capture

`POST /api/photos/capture` takes a JPEG, PNG or WebP as base64 (a `data:` URL
prefix is fine, max 8 MB decoded) and:

1. Auto-rotates it from EXIF and strips all metadata, including any GPS tags.
2. Stamps a band across the bottom: **CapSnap**, a short verification code
   (first 8 characters of the photo id), and table number + capture time (UTC).
   The watermarked copy is capped at 2048 px on the long edge.
3. Makes a 480 px-wide thumbnail of the watermarked image.
4. Generates a random 128-bit review token and a QR PNG that encodes
   `${APP_URL}/r/<token>` (also returned as `reviewUrl`, e.g. for SMS).
5. Uploads everything under `locations/<locationId>/photos/<photoId>/`
   (`original.jpg`, `watermarked.jpg`, `thumbnail.jpg`, `qr.png`) and saves the
   `Photo` record.

Still stubbed: `staffId` is a placeholder until auth lands; photo records live
in memory (`InMemoryPhotoRepository`) and are lost on restart; dish and location
names aren't on the stamp yet because there are no menu/location tables —
`stampSvg` already accepts `dishName`, `locationName` and `timeZone`.

### Photo storage

| `STORAGE_DRIVER` | Behaviour |
|---|---|
| `local` (default) | Writes to `services/api/.data/uploads` (`LOCAL_STORAGE_DIR`) and serves it at `${API_URL}/files/…` |
| `s3` | Uploads to `S3_BUCKET` in `AWS_REGION`; credentials come from the AWS SDK default chain |

The API reads settings from its process environment (it doesn't load `.env`
files), e.g. `STORAGE_DRIVER=s3 S3_BUCKET=nocapsnap-photos pnpm dev:api`.

For S3, keep the bucket private and serve it through CloudFront with Origin
Access Control; set `CLOUDFRONT_DOMAIN` and the returned URLs use it. Without
it, URLs point straight at the bucket and only work if objects are public. If
originals shouldn't be guest-visible, limit the CloudFront behaviour to
`*/watermarked.jpg`, `*/thumbnail.jpg` and `*/qr.png`. `S3_ENDPOINT` targets an
S3-compatible store (MinIO, Cloudflare R2, LocalStack) with path-style URLs.

The watermark text is rendered by sharp through the system's fonts. Slim
server images often have none, so the band would render without text; install
one, e.g. `apt-get install -y fonts-dejavu-core` on Debian-based images.

`pnpm build` bundles `@nocapsnap/shared` into `dist/server.js` with tsup, so
`pnpm --filter @nocapsnap/api start` runs on plain Node.

## Mobile app flow

`index` → `login` (stub) → `capture` (expo-camera) → `preview` → `send` ·
`history` (stub).

`send` uploads the photo, then shows the guest QR (`qrCodeUrl`) on a white card
with the verification code and table, the review link, **Share review link**
(system share sheet — SMS, WhatsApp, etc.) and **Snap next plate** (back to the
camera). If the QR image can't load, it says so and the link/share still work.

The captured photo is held in memory (`src/state/pendingCapture.ts`) between
screens instead of being passed through route params — the base64 image is
several MB.

## Google Play

```bash
cd apps/mobile
npx eas login
npx eas build:configure         # writes the real projectId into app.json
pnpm build:play                 # AAB, production profile
pnpm submit:play                # uploads to the "internal" track
```

Before the first submission:

1. Create the Play Console developer account under **Hammurabi Coding Company LLC**.
2. Let EAS manage the upload keystore (default) or supply your own.
3. Create a Google Cloud service account with Play Console access and save its
   key as `apps/mobile/google-play-service-account.json` (gitignored — never commit it).
4. Run `npx eas build:configure` once — it links the EAS project and writes
   `extra.eas.projectId` into `app.json`. Commit that change.
5. Make sure the Expo account/organization `hammurabicoding` exists (`"owner"` in `app.json`),
   or change `owner` to your Expo username.
6. Replace the placeholder `icon.png`, `adaptive-icon.png` and `splash-icon.png` in
   `apps/mobile/assets/`.
7. Fill in the listing from `apps/mobile/store/play-listing.md`, including a privacy
   policy URL and the Data safety form.

`android/` and `ios/` are generated by `expo prebuild` and are gitignored — EAS
regenerates them from `app.json` on every build, so change native config there.

## Notes

- Expo SDK 57 targets Android API 36, which Google Play requires for new apps and
  updates since Aug 31, 2026. Upgrade the SDK with `npx expo install expo@latest --fix`.
- `.npmrc` uses `node-linker=hoisted`, the layout React Native autolinking and EAS
  Build handle most reliably in a pnpm monorepo.
- `react-native-reanimated`, `react-native-worklets`, `react-native-gesture-handler` and
  `@react-native/metro-config` are pinned in `apps/mobile/package.json` even though no
  screen imports them yet: they are peer dependencies of expo-router's dependencies, and without
  the pins pnpm installs the newest releases instead of the SDK 57–compatible ones.
- Release builds block plain `http://`; production `EXPO_PUBLIC_API_URL` must be `https://`.

## Next implementation pass

- Postgres-backed `PhotoRepository`, plus menu/location lookups so the stamp
  shows dish, restaurant and local time.
- Real staff auth to replace `loginStub` (JWT stored in `expo-secure-store`).
- Dish selector on the capture screen; photo log on `history`.
