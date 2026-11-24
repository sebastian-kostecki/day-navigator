// Searches the whole change folder (file contents and relative paths) for denylist terms.
// Prints counts and file names only, never the terms. Run from the repo root.
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { homedir } from "node:os";
import { join, relative } from "node:path";

const dir = "context/changes/design-system-artifact-sync";
const deny = join(homedir(), ".day-navigator", "push-denylist.txt");
if (!existsSync(deny)) {
  console.log("no-work-data: FAIL (denylist file missing)");
  process.exit(1);
}
const terms = readFileSync(deny, "utf8")
  .split(/\r?\n/)
  .map((l) => l.trim())
  .filter((l) => l && !l.startsWith("#"))
  .map((l) => l.toLowerCase());
if (terms.length === 0) {
  console.log("no-work-data: FAIL (denylist has no terms)");
  process.exit(1);
}

const BINARY = /\.(woff2?|png|ico|icns|jpe?g|gif|webp|ttf|otf)$/i;
const walk = (d) =>
  readdirSync(d, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? walk(join(d, e.name)) : [join(d, e.name)],
  );

const hits = [];
let scanned = 0;
for (const f of walk(dir)) {
  const path = relative(dir, f).toLowerCase();
  const text = BINARY.test(f) ? "" : readFileSync(f, "utf8").toLowerCase();
  scanned++;
  if (terms.some((t) => path.includes(t) || text.includes(t))) hits.push(f);
}
if (hits.length) {
  console.log(`no-work-data: FAIL (${hits.length} file(s) with a denylist hit in name or content)`);
  for (const f of hits) console.log(` - ${f}`);
  process.exit(1);
}
console.log(`no-work-data: OK (${scanned} files and paths scanned, ${terms.length} terms)`);
