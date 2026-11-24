import { existsSync, readFileSync } from "node:fs";

const base = "context/changes/design-system-artifact-sync";
const index = readFileSync("src/components/index.ts", "utf8");
const parts = new Set(["DataTableRow", "DataTableCell"]);

const names = new Set();
for (const m of index.matchAll(/export\s*\{([^}]*)\}\s*from/g)) {
  for (let raw of m[1].split(",")) {
    raw = raw.trim();
    if (!raw || raw.startsWith("type ")) continue;
    if (!/^[A-Z][A-Za-z0-9]*$/.test(raw) || parts.has(raw)) continue;
    if (raw === "ErrorState" || existsSync(`src/components/${raw}`)) names.add(raw);
  }
}

if (names.size !== 18) {
  console.log(`FAIL checklist-coverage: expected 18 components, derived ${names.size}: ${[...names].join(", ")}`);
  process.exit(1);
}

const rows = readFileSync(`${base}/sync-checklist.md`, "utf8")
  .split(/\r?\n/)
  .filter((l) => l.trim().startsWith("|"));
const required = [...names, "RemindersPanel", "SourceCard"];
const missing = required.filter((n) => !rows.some((r) => r.includes("`" + n + "`")));
if (missing.length > 0) {
  console.log(`FAIL checklist-coverage: no table row for ${missing.join(", ")}`);
  process.exit(1);
}
console.log(`PASS checklist-coverage: ${names.size} components + 2 pattern cards covered`);
