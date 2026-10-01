import { encode } from "uqr";
import sharp from "sharp";
const out = process.argv[2];
const url = "https://capsnap.atelier8.example/g/#hNeCjyb44AOdSqzBTJcCTOP0CUq3SGPTvZ2Vcgx3SGPT";
const qr = encode(url, { ecc: "H", border: 0 });
console.log("version", qr.version, "size", qr.size);
const Q = 4, N = qr.size + 2 * Q;
const finders = [[0, 0], [qr.size - 7, 0], [0, qr.size - 7]];
function svg({ dot, finder }) {
  let s = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${-Q} ${-Q} ${N} ${N}" width="${N * 12}" height="${N * 12}"><rect x="${-Q}" y="${-Q}" width="${N}" height="${N}" fill="#FDF0CE"/>`;
  for (const [fx, fy] of finders) {
    if (finder === "round") s += `<rect x="${fx + 0.5}" y="${fy + 0.5}" width="6" height="6" rx="1.6" fill="none" stroke="#1D0E25" stroke-width="1"/><rect x="${fx + 2}" y="${fy + 2}" width="3" height="3" rx="0.7" fill="#1D0E25"/>`;
    else s += `<path d="M${fx} ${fy}h7v7h-7z M${fx + 1} ${fy + 1}v5h5v-5z" fill="#1D0E25" fill-rule="evenodd"/><rect x="${fx + 2}" y="${fy + 2}" width="3" height="3" fill="#1D0E25"/>`;
  }
  qr.data.forEach((row, y) => row.forEach((dark, x) => {
    const t = qr.types[y][x];
    if (!dark || t === 2) return;
    if (t !== 0 || dot === "square") s += `<rect x="${x}" y="${y}" width="1.02" height="1.02" fill="#1D0E25"/>`;
    else if (dot === "roundsq") s += `<rect x="${x + 0.05}" y="${y + 0.05}" width="0.9" height="0.9" rx="0.3" fill="#1D0E25"/>`;
    else s += `<circle cx="${x + 0.5}" cy="${y + 0.5}" r="${dot}" fill="#1D0E25"/>`;
  }));
  return s + "</svg>";
}
const variants = {
  "A-dots46-roundfinder": { dot: 0.46, finder: "round" },
  "B-dots50-roundfinder": { dot: 0.5, finder: "round" },
  "C-roundsq-roundfinder": { dot: "roundsq", finder: "round" },
  "D-dots46-squarefinder": { dot: 0.46, finder: "square" },
  "E-square-squarefinder": { dot: "square", finder: "square" },
};
for (const [name, v] of Object.entries(variants)) await sharp(Buffer.from(svg(v))).png().toFile(`${out}/${name}.png`);
console.log("rendered", Object.keys(variants).length);
