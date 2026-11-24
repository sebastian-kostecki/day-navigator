import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const base = "context/changes/design-system-artifact-sync";
const paths = readFileSync(join(base, "artifact-listing.txt"), "utf8")
  .split(/\r?\n/)
  .map((l) => l.trim())
  .filter(Boolean);

if (paths.length === 0) {
  console.log("FAIL copy-complete: artifact-listing.txt is empty");
  process.exit(1);
}
const missing = paths.filter((p) => !existsSync(join(base, "artifact", p)));
if (missing.length > 0) {
  console.log(`FAIL copy-complete: ${missing.length} of ${paths.length} missing`);
  for (const m of missing) console.log(`  ${m}`);
  process.exit(1);
}
console.log(`PASS copy-complete: ${paths.length} paths present`);
