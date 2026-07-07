import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const iconDir = join("src-tauri", "icons");
mkdirSync(iconDir, { recursive: true });

function makePixels(size) {
  const pixels = Buffer.alloc(size * size * 4);

  function setPixel(x, y, r, g, b, a = 255) {
    const index = (y * size + x) * 4;
    pixels[index] = r;
    pixels[index + 1] = g;
    pixels[index + 2] = b;
    pixels[index + 3] = a;
  }

  function rect(x0, y0, x1, y1, color) {
    const left = Math.round(x0 * size);
    const top = Math.round(y0 * size);
    const right = Math.round(x1 * size);
    const bottom = Math.round(y1 * size);
    for (let y = top; y < bottom; y += 1) {
      for (let x = left; x < right; x += 1) {
        setPixel(x, y, ...color);
      }
    }
  }

  const radius = size * 0.16;
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const inner = x >= size * 0.07 && x <= size * 0.93 && y >= size * 0.07 && y <= size * 0.93;
      const cx = x < size / 2 ? size * 0.07 + radius : size * 0.93 - radius;
      const cy = y < size / 2 ? size * 0.07 + radius : size * 0.93 - radius;
      const inCorner = (x < size * 0.07 + radius || x > size * 0.93 - radius) &&
        (y < size * 0.07 + radius || y > size * 0.93 - radius);
      const rounded = !inCorner || Math.hypot(x - cx, y - cy) <= radius;
      if (!inner || !rounded) {
        setPixel(x, y, 0, 0, 0, 0);
        continue;
      }
      const shade = Math.max(0, Math.min(1, (x + y) / (size * 2)));
      setPixel(x, y, 15 + Math.round(shade * 20), 118 + Math.round(shade * 35), 110 + Math.round(shade * 45));
    }
  }

  rect(0.31, 0.65, 0.41, 0.72, [255, 255, 255, 255]);
  rect(0.59, 0.65, 0.69, 0.72, [255, 255, 255, 255]);
  rect(0.40, 0.36, 0.50, 0.65, [255, 255, 255, 255]);
  rect(0.50, 0.36, 0.60, 0.65, [235, 247, 245, 255]);
  rect(0.36, 0.52, 0.64, 0.59, [255, 255, 255, 255]);

  return pixels;
}

const crcTable = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(buffer) {
  let c = 0xffffffff;
  for (const byte of buffer) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const typeBuffer = Buffer.from(type);
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuffer, data])));
  return Buffer.concat([length, typeBuffer, data, crc]);
}

function zlibStore(data) {
  const chunks = [Buffer.from([0x78, 0x01])];
  for (let offset = 0; offset < data.length; offset += 65535) {
    const block = data.subarray(offset, Math.min(offset + 65535, data.length));
    const final = offset + block.length >= data.length ? 1 : 0;
    const header = Buffer.alloc(5);
    header[0] = final;
    header.writeUInt16LE(block.length, 1);
    header.writeUInt16LE(~block.length & 0xffff, 3);
    chunks.push(header, block);
  }
  let a = 1;
  let b = 0;
  for (const byte of data) {
    a = (a + byte) % 65521;
    b = (b + a) % 65521;
  }
  const adler = Buffer.alloc(4);
  adler.writeUInt32BE(((b << 16) | a) >>> 0);
  chunks.push(adler);
  return Buffer.concat(chunks);
}

function pngFromPixels(size, pixels) {
  const scanlines = Buffer.alloc((size * 4 + 1) * size);
  for (let y = 0; y < size; y += 1) {
    const row = y * (size * 4 + 1);
    scanlines[row] = 0;
    pixels.copy(scanlines, row + 1, y * size * 4, (y + 1) * size * 4);
  }

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", zlibStore(scanlines)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

const pngs = new Map();
for (const size of [16, 32, 64, 128, 256, 512, 1024]) {
  const png = pngFromPixels(size, makePixels(size));
  pngs.set(size, png);
  writeFileSync(join(iconDir, `${size}x${size}.png`), png);
}

writeFileSync(join(iconDir, "icon.png"), pngs.get(256));

function icoEntry(size, png, offset) {
  const entry = Buffer.alloc(16);
  entry[0] = size >= 256 ? 0 : size;
  entry[1] = size >= 256 ? 0 : size;
  entry[2] = 0;
  entry[3] = 0;
  entry.writeUInt16LE(1, 4);
  entry.writeUInt16LE(32, 6);
  entry.writeUInt32LE(png.length, 8);
  entry.writeUInt32LE(offset, 12);
  return entry;
}

const icoSizes = [16, 32, 64, 128, 256];
const icoHeader = Buffer.alloc(6);
icoHeader.writeUInt16LE(0, 0);
icoHeader.writeUInt16LE(1, 2);
icoHeader.writeUInt16LE(icoSizes.length, 4);
let offset = 6 + icoSizes.length * 16;
const icoEntries = [];
for (const size of icoSizes) {
  const png = pngs.get(size);
  icoEntries.push(icoEntry(size, png, offset));
  offset += png.length;
}
writeFileSync(
  join(iconDir, "icon.ico"),
  Buffer.concat([icoHeader, ...icoEntries, ...icoSizes.map((size) => pngs.get(size))]),
);

function icnsEntry(type, data) {
  const header = Buffer.alloc(8);
  header.write(type, 0, 4, "ascii");
  header.writeUInt32BE(data.length + 8, 4);
  return Buffer.concat([header, data]);
}

const icnsEntries = [
  ["icp4", pngs.get(16)],
  ["ic11", pngs.get(32)],
  ["icp5", pngs.get(32)],
  ["ic12", pngs.get(64)],
  ["ic07", pngs.get(128)],
  ["ic13", pngs.get(256)],
  ["ic08", pngs.get(256)],
  ["ic14", pngs.get(512)],
  ["ic09", pngs.get(512)],
  ["ic10", pngs.get(1024)],
].map(([type, data]) => icnsEntry(type, data));
const icnsBody = Buffer.concat(icnsEntries);
const icnsHeader = Buffer.alloc(8);
icnsHeader.write("icns", 0, 4, "ascii");
icnsHeader.writeUInt32BE(icnsBody.length + 8, 4);
writeFileSync(join(iconDir, "icon.icns"), Buffer.concat([icnsHeader, icnsBody]));
