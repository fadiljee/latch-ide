// Memeriksa anggaran ukuran bundel gzipped sesuai docs/ide/IDE_NFR_BUDGET.md (IT-007).
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";

const distDir = path.resolve(import.meta.dirname, "..", "dist");
const MAX_INITIAL_GZIP = 600 * 1024;
const MAX_TOTAL_GZIP = 3 * 1024 * 1024;

function walk(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...walk(full));
    } else if (entry.isFile() && /\.(js|css|html)$/.test(entry.name)) {
      out.push(full);
    }
  }
  return out;
}

if (!fs.existsSync(distDir)) {
  console.error(`dist directory missing: ${distDir}`);
  process.exit(1);
}

let totalGzip = 0;
for (const file of walk(distDir)) {
  const raw = fs.readFileSync(file);
  const gz = zlib.gzipSync(raw, { level: 9 }).length;
  totalGzip += gz;
  const rel = path.relative(distDir, file);
  console.log(`  ${rel}: raw=${raw.length}B gzip=${gz}B`);
}

console.log(`Total gzipped bundle: ${totalGzip}B (limit initial=${MAX_INITIAL_GZIP}B, total=${MAX_TOTAL_GZIP}B)`);
if (totalGzip > MAX_INITIAL_GZIP || totalGzip > MAX_TOTAL_GZIP) {
  console.error("Bundle size budget exceeded!");
  process.exit(1);
}
