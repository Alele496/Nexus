// Generates resources/tray.png — a 32x32 indigo "N" glyph, pure Node (no deps).
// Run once: `node scripts/gen-tray.js` after `npm run build` (tsc emits it).

import { deflateSync } from 'node:zlib';
import fs from 'node:fs';
import path from 'node:path';

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(buf: Buffer): number {
  let c = 0xffffffff;
  for (const byte of buf) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type: string, data: Buffer): Buffer {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, 'ascii');
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crc]);
}

function pngEncode(width: number, height: number, rgba: Buffer): Buffer {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type: RGBA
  const stride = 1 + width * 4;
  const raw = Buffer.alloc(height * stride);
  for (let y = 0; y < height; y++) {
    raw[y * stride] = 0; // filter: none
    rgba.copy(raw, y * stride + 1, y * width * 4, (y + 1) * width * 4);
  }
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

const SIZE = 32;
// Project-root resources/ (committed asset, packaged by electron-builder).
const OUT = path.join(__dirname, '..', '..', 'resources', 'tray.png');

// indigo-500 #6366f1 on transparent, rounded corners r=7, white "N" glyph.
function draw(): Buffer {
  const px = Buffer.alloc(SIZE * SIZE * 4); // zero = transparent
  const set = (x: number, y: number, r: number, g: number, b: number, a: number) => {
    const i = (y * SIZE + x) * 4;
    px[i] = r;
    px[i + 1] = g;
    px[i + 2] = b;
    px[i + 3] = a;
  };
  const inRoundCorner = (x: number, y: number) => {
    const r = 7;
    const corners: Array<[number, number]> = [
      [r, r],
      [SIZE - 1 - r, r],
      [r, SIZE - 1 - r],
      [SIZE - 1 - r, SIZE - 1 - r],
    ];
    return corners.some(([cx, cy]) => (x - cx) ** 2 + (y - cy) ** 2 > r * r);
  };
  const bg = [99, 102, 241]; // indigo-500
  const fg = [255, 255, 255];
  for (let y = 0; y < SIZE; y++) {
    for (let x = 0; x < SIZE; x++) {
      if (inRoundCorner(x, y)) continue;
      // "N": left bar x 7..10, right bar x 21..24, 2px diagonal between them.
      const diagX = 10 + Math.round(((y - 7) * 11) / 18);
      const isGlyph =
        (x >= 7 && x <= 10 && y >= 7 && y <= 25) ||
        (x >= 21 && x <= 24 && y >= 7 && y <= 25) ||
        (y >= 7 && y <= 25 && Math.abs(x - diagX) <= 1);
      if (isGlyph) set(x, y, fg[0], fg[1], fg[2], 255);
      else set(x, y, bg[0], bg[1], bg[2], 255);
    }
  }
  return pngEncode(SIZE, SIZE, px);
}

fs.mkdirSync(path.dirname(OUT), { recursive: true });
fs.writeFileSync(OUT, draw());
console.log(`wrote ${OUT} (${fs.statSync(OUT).size} bytes)`);
