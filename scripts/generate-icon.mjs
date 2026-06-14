// Dependency-free PNG icon generator for Nexus Notes.
// Produces a 1024x1024 RGBA source icon (rounded violet tile + a small "nexus"
// node graph). `pnpm tauri icon` then derives every platform icon from it.
import { deflateSync, crc32 } from "node:zlib";
import { writeFileSync } from "node:fs";

const S = 1024;
const buf = Buffer.alloc(S * S * 4);

const set = (x, y, r, g, b, a) => {
  const i = (y * S + x) * 4;
  // alpha-over composite onto existing pixel
  const ea = buf[i + 3] / 255;
  const na = a / 255;
  const out = na + ea * (1 - na);
  if (out <= 0) return;
  buf[i] = (r * na + buf[i] * ea * (1 - na)) / out;
  buf[i + 1] = (g * na + buf[i + 1] * ea * (1 - na)) / out;
  buf[i + 2] = (b * na + buf[i + 2] * ea * (1 - na)) / out;
  buf[i + 3] = out * 255;
};

const lerp = (a, b, t) => a + (b - a) * t;
const inRoundedRect = (x, y, r) => {
  const cx = Math.min(Math.max(x, r), S - 1 - r);
  const cy = Math.min(Math.max(y, r), S - 1 - r);
  return Math.hypot(x - cx, y - cy) <= r;
};

// Background: rounded tile with a vertical violet gradient.
for (let y = 0; y < S; y++) {
  const t = y / S;
  const r = Math.round(lerp(109, 70, t));
  const g = Math.round(lerp(92, 60, t));
  const b = Math.round(lerp(255, 200, t));
  for (let x = 0; x < S; x++) {
    if (inRoundedRect(x, y, 210)) set(x, y, r, g, b, 255);
  }
}

const nodes = [
  [340, 420, 78],
  [710, 350, 60],
  [560, 720, 66],
];
const edges = [
  [0, 1],
  [0, 2],
  [1, 2],
];

// Edges (semi-transparent white lines).
const distToSeg = (px, py, ax, ay, bx, by) => {
  const dx = bx - ax, dy = by - ay;
  const len2 = dx * dx + dy * dy || 1;
  let t = ((px - ax) * dx + (py - ay) * dy) / len2;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
};
for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    for (const [a, b] of edges) {
      if (distToSeg(x, y, nodes[a][0], nodes[a][1], nodes[b][0], nodes[b][1]) <= 13) {
        set(x, y, 255, 255, 255, 150);
        break;
      }
    }
  }
}

// Nodes (solid white circles with a soft edge).
for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    for (const [nx, ny, nr] of nodes) {
      const d = Math.hypot(x - nx, y - ny);
      if (d <= nr) {
        const a = d > nr - 3 ? 255 * (nr - d) / 3 : 255;
        set(x, y, 255, 255, 255, a);
      }
    }
  }
}

// --- PNG encode ---
const chunk = (type, data) => {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const td = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(td) >>> 0, 0);
  return Buffer.concat([len, td, crc]);
};

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0);
ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA
// rest zero (compression, filter, interlace)

const raw = Buffer.alloc((S * 4 + 1) * S);
for (let y = 0; y < S; y++) {
  raw[y * (S * 4 + 1)] = 0; // filter: none
  buf.copy(raw, y * (S * 4 + 1) + 1, y * S * 4, (y + 1) * S * 4);
}

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

writeFileSync(new URL("../app-icon.png", import.meta.url), png);
console.log("wrote app-icon.png", png.length, "bytes");
