// Compares src/styles/tokens.css :root with the artifact tokens.json. Run from the repo root.
import { readFileSync, existsSync } from "node:fs";

const CHANGE = "context/changes/design-system-artifact-sync";
const css = readFileSync("src/styles/tokens.css", "utf8");
const json = JSON.parse(readFileSync(`${CHANGE}/artifact/project/tokens.json`, "utf8"));
const errors = [];

const root = css.match(/:root\s*\{([\s\S]*?)\n\}/)[1].replace(/\/\*[\s\S]*?\*\//g, "");
const raw = new Map();
for (const m of root.matchAll(/--([\w-]+)\s*:\s*([^;]+);/g)) raw.set(m[1], m[2].trim());
const SKIP = new Set(["hatch-bg", "font-sans", "font-mono"]);
const resolveCss = (v, d = 0) =>
  d > 10 ? v : v.replace(/var\(--([\w-]+)\)/g, (_, n) => resolveCss(raw.get(n) ?? `?${n}`, d + 1));
const norm = (v) => String(v).trim().toLowerCase();

const cssTokens = new Map();
for (const [n, v] of raw) if (!SKIP.has(n)) cssTokens.set(n, norm(resolveCss(v)));

const jsonTokens = new Map();
const dark = new Map();
for (const t of json.color.tokens) dark.set(t.name, t.value.dark);
const resolveAlias = (v, d = 0) => {
  const m = /^\{([\w-]+)\}$/.exec(v);
  return m && d < 10 ? resolveAlias(dark.get(m[1]) ?? `?${m[1]}`, d + 1) : v;
};
const add = (n, v) => {
  if (jsonTokens.has(n)) errors.push(`duplicate token name: ${n}`);
  jsonTokens.set(n, norm(v));
};
for (const t of json.color.tokens) add(t.name, resolveAlias(t.value.dark));
for (const fam of ["spacing", "radius", "size"]) for (const t of json[fam].tokens) add(t.name, t.value);

for (const [n, v] of cssTokens) {
  if (!jsonTokens.has(n)) errors.push(`missing in tokens.json: ${n}`);
  else if (jsonTokens.get(n) !== v) errors.push(`value differs: ${n} css=${v} json=${jsonTokens.get(n)}`);
}
for (const n of jsonTokens) if (!cssTokens.has(n[0])) errors.push(`not in tokens.css: ${n[0]}`);

const fonts = json.type?.fonts ?? [];
if (fonts.length === 0) errors.push("type.fonts is empty");
for (const f of fonts) {
  const name = f.file.replace(/^fonts\//, "");
  if (!existsSync(`src/assets/fonts/${name}`)) errors.push(`font file missing in src/assets/fonts: ${name}`);
}

if (errors.length) {
  console.log(`tokens-parity: FAIL (${errors.length})`);
  for (const e of errors) console.log(` - ${e}`);
  process.exit(1);
}
console.log(`tokens-parity: OK (${cssTokens.size} tokens, ${fonts.length} fonts)`);
