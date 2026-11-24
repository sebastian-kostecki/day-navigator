// Builds one static HTML page per artifact card preview (plus index.html) for a local look before publishing.
// Usage (repo root): node context/changes/design-system-artifact-sync/checks/render-local.mjs <outDir>
import { readFileSync, writeFileSync, mkdirSync, readdirSync, existsSync } from "node:fs";
import { resolve, join } from "node:path";
import { pathToFileURL } from "node:url";

const PROJECT = "context/changes/design-system-artifact-sync/artifact/project";
const REACT = "https://cdn.jsdelivr.net/npm/react@18.3.1/umd/react.production.min.js";
const REACT_DOM = "https://cdn.jsdelivr.net/npm/react-dom@18.3.1/umd/react-dom.production.min.js";

const fail = (msg) => {
  console.error(`render-local: ${msg}`);
  process.exit(1);
};
const outArg = process.argv[2];
if (!outArg) fail("usage: node render-local.mjs <outDir>");
const outDir = resolve(outArg);
mkdirSync(outDir, { recursive: true });

const tokens = JSON.parse(readFileSync(`${PROJECT}/tokens.json`, "utf8"));
const bundleCss = readFileSync(`${PROJECT}/components/bundle.css`, "utf8");
const bundleJs = readFileSync(`${PROJECT}/components/bundle.js`, "utf8");
if (/<\/script/i.test(bundleJs)) fail("bundle.js contains </script");

// --- tokens CSS, as the artifact compiler would emit it ---
const theme = tokens.color.themes[0].id;
const val = (v) => v.replace(/\{([\w-]+)\}/g, "var(--$1)");
const lines = [];
lines.push(`:root, [data-theme="${theme}"] {`);
for (const t of tokens.color.tokens) lines.push(`  --${t.name}: ${val(t.value[theme])};`);
lines.push("}", ":root {");
for (const k of ["spacing", "radius", "size"]) for (const t of tokens[k].tokens) lines.push(`  --${t.name}: ${t.value};`);
for (const [k, v] of Object.entries(tokens.type.families)) lines.push(`  --font-${k}: ${v};`);
lines.push("}");
const props = ["fontSize", "lineHeight", "fontWeight", "letterSpacing"];
const kebab = (s) => s.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());
const styleNames = [];
for (const g of tokens.type.groups)
  for (const s of g.styles) {
    styleNames.push(s.name);
    const d = [`font-family: var(--font-${g.family})`];
    for (const p of props) if (s[p] !== undefined) d.push(`${kebab(p)}: ${s[p]}`);
    lines.push(`.${s.name} { ${d.join("; ")}; }`);
  }
const fontDir = resolve("src/assets/fonts");
for (const f of tokens.type.fonts) {
  const file = f.file.split("/").pop();
  const url = pathToFileURL(join(fontDir, file)).href;
  if (!existsSync(join(fontDir, file))) console.warn(`WARN: font file missing: ${file}`);
  lines.push(
    `@font-face { font-family: "${f.family}"; font-weight: ${f.weight}; font-style: ${f.style}; src: url("${url}") format("woff2"); }`,
  );
}
const tokensCss = lines.join("\n");

// --- WARN: app text-style CSS the compiler cannot express ---
{
  const app = readFileSync("src/styles/tokens.css", "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  const ok = new Set(["font-family", "font-size", "line-height", "font-weight", "letter-spacing"]);
  const extra = new Map();
  for (const m of app.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const sels = m[1].split(",").map((s) => s.trim());
    const decls = [...m[2].matchAll(/([\w-]+)\s*:/g)].map((x) => x[1]).filter((p) => !p.startsWith("--") && !ok.has(p));
    for (const s of sels) {
      const n = s.startsWith(".") ? s.slice(1) : null;
      if (n && styleNames.includes(n) && decls.length) extra.set(n, [...(extra.get(n) ?? []), ...decls]);
    }
  }
  for (const [n, d] of extra) console.log(`WARN: .${n} has app-only properties the compiler cannot express: ${[...new Set(d)].join(", ")}`);
}

// --- pages ---
const compDir = `${PROJECT}/components`;
const names = readdirSync(compDir, { withFileTypes: true }).filter((e) => e.isDirectory() && existsSync(`${compDir}/${e.name}/preview.html`)).map((e) => e.name);
const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");
const index = [];
for (const name of names) {
  const raw = readFileSync(`${compDir}/${name}/preview.html`, "utf8").replace(/^\uFEFF/, "");
  const nl = raw.indexOf("\n");
  const first = nl < 0 ? raw : raw.slice(0, nl);
  const mk = first.match(/^<!--\s*@dsCard\b([\s\S]*?)-->\s*$/);
  if (!mk) fail(`${name}: line 1 is not an @dsCard marker`);
  const group = mk[1].match(/group="([^"]*)"/)?.[1] ?? "";
  const height = mk[1].match(/height=(\d+)/)?.[1] ?? "";
  const rest = raw.slice(nl + 1);
  const body = rest.match(/<body[^>]*>([\s\S]*)<\/body>/i);
  if (!body) fail(`${name}: preview has no <body>`);
  const head = rest.match(/<head[^>]*>([\s\S]*?)<\/head>/i)?.[1] ?? "";
  const headStyles = [...head.matchAll(/<style[^>]*>[\s\S]*?<\/style>/gi)].map((m) => m[0]).join("\n");
  const title = head.match(/<title>([\s\S]*?)<\/title>/i)?.[1] ?? name;
  const html = `<!doctype html>
<html data-theme="${theme}">
<head>
<meta charset="utf-8">
<title>${title}</title>
<style>
${tokensCss}
${bundleCss}
</style>
${headStyles}
</head>
<body>
<script src="${REACT}"></script>
<script src="${REACT_DOM}"></script>
<script>
${bundleJs}
</script>
${body[1]}
</body>
</html>
`;
  writeFileSync(join(outDir, `${name}.html`), html);
  index.push({ name, group, height });
}
const li = index.map((c) => `<li><a href="${c.name}.html">${esc(c.name)}</a> — group: ${esc(c.group || "(none)")}, height: ${c.height || "?"}</li>`).join("\n");
writeFileSync(join(outDir, "index.html"), `<!doctype html>\n<html data-theme="${theme}"><head><meta charset="utf-8"><title>Local render</title></head><body style="font-family:sans-serif"><ul>\n${li}\n</ul></body></html>\n`);
console.log(`render-local: built ${index.length} pages -> ${outDir}`);
