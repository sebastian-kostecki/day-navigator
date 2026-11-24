// Searches the artifact working copy for denylist terms. Prints counts and file names only. Run from the repo root.
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join } from "node:path";

const dir = "context/changes/design-system-artifact-sync/artifact";
const deny = join(process.env.USERPROFILE ?? "", ".day-navigator", "push-denylist.txt");
if (!existsSync(deny)) {
  console.log("no-work-data: FAIL (denylist file missing)");
  process.exit(1);
}
const terms = readFileSync(deny, "utf8")
  .split(/\r?\n/)
  .map((l) => l.trim())
  .filter((l) => l && !l.startsWith("#"))
  .map((l) => l.toLowerCase());

const BINARY = /\.(woff2?|png|ico|icns|jpe?g|gif|webp|ttf|otf)$/i;
const walk = (d) =>
  readdirSync(d, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? walk(join(d, e.name)) : [join(d, e.name)],
  );

const hits = [];
let scanned = 0;
for (const f of walk(dir)) {
  if (BINARY.test(f)) continue;
  scanned++;
  const text = readFileSync(f, "utf8").toLowerCase();
  if (terms.some((t) => text.includes(t))) hits.push(f);
}
if (hits.length) {
  console.log(`no-work-data: FAIL (${hits.length} file(s) with a denylist hit)`);
  for (const f of hits) console.log(` - ${f}`);
  process.exit(1);
}
console.log(`no-work-data: OK (${scanned} files scanned)`);
