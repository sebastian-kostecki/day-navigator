// Checks the component cards of the artifact working copy (run from the repo root, plain Node, no deps).
// Cards: the 18 names in the bundle.js `@ds-bundle` header plus the two pattern cards.
// Per card: README.md starts with "# <Name>"; preview.html line 1 is a valid @dsCard marker
// (quoted group, numeric height 40..4000, bare `page` flag iff pattern card) and the file has no
// iframe/object/embed/frame/portal/noscript, no http(s) URL, no `import`, no `useId`, no hex colour.
//
// Hex rule: a match of #[0-9a-f]{3,8} (not preceded by a word char or '&') is a colour when
// (a) it has an a-f letter (#27 and #1203 issue refs never do) or
// (b) it is digits only but sits in a CSS value position: right after `:`, `(` or `,` (whitespace
//     allowed) and right before `;`, `)`, `,`, `}` or a quote. A quoted ref such as '#1203' is not a colour.
import { readFileSync, existsSync } from 'node:fs';

const DIR = 'context/changes/design-system-artifact-sync/artifact/project/components';
const PATTERN_CARDS = ['RemindersPanel', 'SourceCard'];
const FORBIDDEN_TAGS = ['iframe', 'object', 'embed', 'frame', 'portal', 'noscript'];

const fails = [];
const fail = (name, msg) => fails.push(`${name}: ${msg}`);

const bundle = readFileSync(`${DIR}/bundle.js`, 'utf8');
const header = /\/\*\s*@ds-bundle:\s*(\{.*?\})\s*\*\//.exec(bundle);
if (!header) {
  console.log('cards-check: FAIL (no @ds-bundle header in bundle.js)');
  process.exit(1);
}
const exported = JSON.parse(header[1]).components.map((c) => c.name);
const names = [...exported, ...PATTERN_CARDS];

function hexColours(text) {
  const out = [];
  const re = /#[0-9a-fA-F]{3,8}\b/g;
  let m;
  while ((m = re.exec(text))) {
    const before = text[m.index - 1];
    if (before && /[\w&]/.test(before)) continue;
    const tok = m[0].slice(1);
    if (![3, 4, 6, 8].includes(tok.length)) continue;
    if (/[a-fA-F]/.test(tok)) {
      out.push(m[0]);
      continue;
    }
    const pre = text.slice(0, m.index);
    const post = text.slice(m.index + m[0].length);
    if (/[:(,]\s*$/.test(pre) && /^\s*[;),}'"]/.test(post)) out.push(m[0]);
  }
  return out;
}

for (const name of names) {
  const readme = `${DIR}/${name}/README.md`;
  const preview = `${DIR}/${name}/preview.html`;
  if (!existsSync(readme)) fail(name, 'README.md missing');
  else if (!readFileSync(readme, 'utf8').startsWith(`# ${name}\n`)) fail(name, `README.md must start with "# ${name}"`);

  if (!existsSync(preview)) {
    fail(name, 'preview.html missing');
    continue;
  }
  const text = readFileSync(preview, 'utf8');
  const first = text.split(/\r?\n/, 1)[0];
  if (!/^<!--\s*@dsCard\b/.test(first) || !/-->\s*$/.test(first)) {
    fail(name, 'line 1 is not an @dsCard marker');
  } else {
    const group = /\bgroup="([^"]+)"/.exec(first);
    const height = /\bheight=(\d+)\b/.exec(first);
    if (!group) fail(name, 'marker has no quoted group');
    if (!height || +height[1] < 40 || +height[1] > 4000) fail(name, 'marker height missing or outside 40..4000');
    const bare = first.replace(/"[^"]*"/g, '""').replace(/^<!--\s*@dsCard/, '').replace(/-->\s*$/, '');
    const hasPage = /(^|\s)page(\s|$)/.test(bare);
    const isPattern = PATTERN_CARDS.includes(name);
    if (isPattern && !hasPage) fail(name, 'pattern card needs the bare `page` flag');
    if (!isPattern && hasPage) fail(name, 'only pattern cards may have the `page` flag');
    if (isPattern && group && group[1] !== 'Wzorce widoków') fail(name, 'pattern card group must be "Wzorce widoków"');
  }
  for (const tag of FORBIDDEN_TAGS) if (new RegExp(`<${tag}\\b`, 'i').test(text)) fail(name, `contains <${tag}`);
  if (/https?:\/\//i.test(text)) fail(name, 'contains an http(s) URL');
  if (/^\s*import\s/m.test(text)) fail(name, 'contains import');
  if (/\buseId\b/.test(text)) fail(name, 'uses useId (React 18 previews)');
  const hex = hexColours(text);
  if (hex.length) fail(name, `hex colour literal(s): ${[...new Set(hex)].join(' ')}`);
}

if (fails.length) {
  console.log(`cards-check: FAIL (${fails.length})`);
  for (const f of fails) console.log(` - ${f}`);
  process.exit(1);
}
console.log(`cards-check: OK (${names.length} cards: ${exported.length} components + ${PATTERN_CARDS.length} patterns)`);
