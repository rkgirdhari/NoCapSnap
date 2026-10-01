import sharp from "sharp";
const M = "$REPO/nocapsnap/.hcc-nocapsnap/design/mockups", O = "$SCRATCH/w2/fix";
const crops = [
  ["cod.jpg", "03-review-plate-as-served.webp", { left: 44, top: 374, width: 1038, height: 976 }],
  ["risotto.jpg", "02-prepare-which-dish.webp", { left: 82, top: 930, width: 226, height: 174 }],
  ["carrots.jpg", "02-prepare-which-dish.webp", { left: 82, top: 1174, width: 226, height: 174 }],
];
for (const [name, src, box] of crops) {
  const info = await sharp(`${M}/${src}`).extract(box).resize({ width: Math.max(box.width, 1200) }).jpeg({ quality: 90 }).toFile(`${O}/${name}`);
  console.log(name, info.width + "x" + info.height, info.size);
}
