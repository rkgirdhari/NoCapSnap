import sharp from "sharp";
const M = "$REPO/nocapsnap/.hcc-nocapsnap/design/mockups", I = "$SCRATCH/w2/shots", O = "$SCRATCH/w2/side";
const pairs = [
  ["01-home.webp", "01-home.png", "01-home"],
  ["02-prepare-which-dish.webp", "02-prepare.png", "02-prepare"],
  ["03-review-plate-as-served.webp", "03-review.png", "03-review"],
  ["04-history-offline.webp", "04-history-offline.png", "04-history-offline"],
  ["05-guest-invitation-qr.webp", "05-invite-synced.png", "05-guest-invitation"],
];
const H = 1334; // both at 2/3 of the 2000 px mockup height
for (const [m, s, name] of pairs) {
  const a = await sharp(`${M}/${m}`).resize({ height: H }).toBuffer();
  const b = await sharp(`${I}/${s}`).resize({ height: H }).toBuffer();
  const wa = (await sharp(a).metadata()).width, wb = (await sharp(b).metadata()).width;
  const gap = 24;
  await sharp({ create: { width: wa + wb + gap, height: H, channels: 3, background: "#0b0410" } })
    .composite([{ input: a, left: 0, top: 0 }, { input: b, left: wa + gap, top: 0 }])
    .webp({ quality: 78 })
    .toFile(`${O}/${name}.webp`);
}
console.log("ok");
