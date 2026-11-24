// Parity of bundle.css and index.d.ts with the app (run from the repo root).
import fs from 'node:fs';
import path from 'node:path';

const BASE = 'context/changes/design-system-artifact-sync/artifact/project/components';
const norm = (s) => s.replace(/\s+/g, ' ').trim();
let failed = 0;
const report = (ok, name, items = []) => {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
  if (!ok) {
    failed++;
    for (const i of items) console.log('  - ' + i);
  }
};

const bundle = norm(fs.readFileSync(`${BASE}/bundle.css`, 'utf8'));
const cssFiles = fs
  .readdirSync('src/components', { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .flatMap((d) =>
    fs
      .readdirSync(path.join('src/components', d.name))
      .filter((f) => f.endsWith('.css'))
      .map((f) => path.join('src/components', d.name, f)),
  );
const missingCss = cssFiles.filter((f) => !bundle.includes(norm(fs.readFileSync(f, 'utf8'))));
report(missingCss.length === 0 && cssFiles.length > 0, `component CSS copied verbatim (${cssFiles.length} files)`, missingCss);

const hex = fs.readFileSync(`${BASE}/bundle.css`, 'utf8').match(/#[0-9a-fA-F]{3,8}\b/g) || [];
report(hex.length === 0, 'no hex colour literals in bundle.css', hex);

const indexTs = fs.readFileSync('src/components/index.ts', 'utf8');
const types = new Set();
for (const blk of indexTs.matchAll(/export\s*\{([^}]*)\}/g)) {
  for (const m of blk[1].matchAll(/\btype\s+(\w+)/g)) types.add(m[1]);
}
const dts = fs.readFileSync(`${BASE}/index.d.ts`, 'utf8');
const missingTypes = [...types].filter((t) => !new RegExp(`(?:export\\s+)?(?:interface|type)\\s+${t}\\b`).test(dts));
report(missingTypes.length === 0 && types.size > 0, `index.d.ts declares all ${types.size} exported types`, missingTypes);

process.exit(failed ? 1 : 0);
