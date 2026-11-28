// Memeriksa anggaran ukuran bundel gzipped sesuai docs/ide/IDE_NFR_BUDGET.md (IT-007, IT-117).
// Memisahkan bundel initial load (direferensikan index.html <= 600 KiB) dan total seluruh chunk lazy (<= 3 MiB).
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

function collectInitialRelPaths(indexHtml) {
  const refs = new Set(["index.html"]);
  const attrRegex = /(?:src|href)="\/?([^"]+)"/g;
  for (const match of indexHtml.matchAll(attrRegex)) {
    const ref = match[1];
    if (ref && /\.(js|css)$/.test(ref)) {
      refs.add(ref);
    }
  }
  return refs;
}

if (!fs.existsSync(distDir)) {
  console.error(`dist directory missing: ${distDir}`);
  process.exit(1);
}

const indexPath = path.join(distDir, "index.html");
const initialRefs = fs.existsSync(indexPath)
  ? collectInitialRelPaths(fs.readFileSync(indexPath, "utf8"))
  : new Set();

let initialGzip = 0;
let totalGzip = 0;

for (const file of walk(distDir)) {
  const raw = fs.readFileSync(file);
  const gz = zlib.gzipSync(raw, { level: 9 }).length;
  const rel = path.relative(distDir, file).split(path.sep).join("/");
  totalGzip += gz;
  const isInitial = initialRefs.has(rel);
  if (isInitial) {
    initialGzip += gz;
  }
  console.log(`  ${rel}: raw=${raw.length}B gzip=${gz}B${isInitial ? " [initial]" : ""}`);
}

console.log(
  `Bundle budget: initial=${initialGzip}B (limit=${MAX_INITIAL_GZIP}B), total=${totalGzip}B (limit=${MAX_TOTAL_GZIP}B)`,
);
if (initialGzip > MAX_INITIAL_GZIP) {
  console.error(`Initial gzipped bundle exceeded budget: ${initialGzip}B > ${MAX_INITIAL_GZIP}B`);
  process.exit(1);
}
if (totalGzip > MAX_TOTAL_GZIP) {
  console.error(`Total gzipped bundle exceeded budget: ${totalGzip}B > ${MAX_TOTAL_GZIP}B`);
  process.exit(1);
}
