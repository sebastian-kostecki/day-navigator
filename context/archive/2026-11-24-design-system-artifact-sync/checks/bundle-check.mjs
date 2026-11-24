// Sanity check of artifact/project/components/bundle.js against the app's TSX (run from the repo root).
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import vm from 'node:vm';

const require = createRequire(import.meta.url);
const React = require(require.resolve('react', { paths: [process.cwd()] }));
const { renderToStaticMarkup } = require(require.resolve('react-dom/server', { paths: [process.cwd()] }));

const FILE = 'context/changes/design-system-artifact-sync/artifact/project/components/bundle.js';
const NAMES = [
  'AppHeader', 'Button', 'Chip', 'DataTable', 'ErrorState', 'Field', 'Flag', 'HoursBar', 'Icon',
  'ItemRow', 'Notice', 'Panel', 'Segmented', 'SourceChip', 'SourceState', 'StatStrip', 'StatusPill', 'Tabs',
];
let failed = 0;
const report = (ok, name, detail) => {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${ok || !detail ? '' : ' - ' + detail}`);
  if (!ok) failed++;
};

const src = fs.readFileSync(FILE, 'utf8');

// 1 syntax
try {
  execFileSync(process.execPath, ['--check', FILE], { stdio: 'pipe' });
  report(true, 'syntax (node --check)');
} catch (e) {
  report(false, 'syntax (node --check)', String(e.stderr || e.message).split('\n')[0]);
}

// 2 forbidden text
const forbidden = [
  ['</script', /<\/script/i],
  ['<!--', /<!--/],
  ['line-start import', /^\s*import\s/m],
  ['fetch(', /fetch\(/],
];
for (const [label, re] of forbidden) report(!re.test(src), `no ${label}`);

// 3 header
const m = src.match(/^\/\* @ds-bundle: (\{.*\}) \*\//);
try {
  const hdr = JSON.parse(m[1]);
  const headerNames = hdr.components.map((c) => c.name);
  report(
    hdr.format === 4 && hdr.namespace === 'DayNav' && JSON.stringify(headerNames) === JSON.stringify(NAMES),
    'header lists the 18 components',
  );
} catch {
  report(false, 'header lists the 18 components', 'cannot parse header');
}

// 4 load
const win = { React };
const ctx = vm.createContext({ window: win, console });
let DN = {};
try {
  vm.runInContext(src, ctx, { filename: 'bundle.js' });
  DN = win.DayNav;
  const want = [...NAMES, 'DataTableRow', 'DataTableCell'];
  const missing = want.filter((n) => !DN[n]);
  const extra = Object.keys(DN).filter((n) => !want.includes(n));
  report(missing.length === 0 && extra.length === 0, 'window.DayNav exports', `missing=${missing} extra=${extra}`);
} catch (e) {
  report(false, 'window.DayNav exports', e.message);
}

// 5 render samples: [name, build, expected class set, substrings that must appear, substrings that must not]
const h = React.createElement;
const noop = () => {};
const samples = [
  ['AppHeader',
    () => h(DN.AppHeader, {
      items: [{ id: 'plan', label: 'Plan', active: true, key: '1' }, { id: 'tasks', label: 'Zadania', active: false }],
      onSelect: noop,
    }, h('div', { className: 'slot' }, 'x')),
    'dn-header dn-brand dn-nav dn-nav-item is-active dn-nav-key slot',
    ['aria-current="page"', 'aria-keyshortcuts="1"', 'data-view="plan"', 'aria-hidden="true"']],
  ['Button',
    () => h(DN.Button, { variant: 'primary', size: 'lg', iconOnly: true, icon: 'plus', pressed: true, className: 'extra', 'aria-label': 'Dodaj' }),
    'dn-btn dn-btn-primary dn-btn-lg dn-btn-icon is-pressed extra dn-icon',
    ['type="button"', 'aria-pressed="true"']],
  ['Button (default)', () => h(DN.Button, null, 'OK'), 'dn-btn dn-btn-secondary dn-btn-md', ['type="button"'], ['aria-pressed']],
  ['Chip', () => h(DN.Chip, { tone: 'warn', className: 'x' }, 'P1'), 'dn-chip dn-chip-warn x'],
  ['DataTable',
    () => h(DN.DataTable, { label: 'Dni', className: 'cols' },
      h(DN.DataTableRow, { head: true }, h(DN.DataTableCell, { header: true }, 'Dzień')),
      h(DN.DataTableRow, { current: true, hatched: true, className: 'r' },
        h(DN.DataTableCell, { tone: 'info', truncate: true, title: 't' }, '9:00'),
        h(DN.DataTableCell, { tone: 'warn' }, '1:00'))),
    'dn-table cols dn-table-row dn-table-head label-caps dn-table-body is-current is-hatched r dn-table-cell dn-table-cell-info dn-table-cell-truncate dn-table-cell-warn',
    ['role="table"', 'role="columnheader"', 'role="cell"', 'aria-current="true"']],
  ['ErrorState',
    () => h(DN.ErrorState, { title: 'Błąd', message: 'Nie działa', onRetry: noop, lastSuccess: 'ostatnio OK 10:00' }),
    'dn-state dn-state-error dn-icon dn-state-icon dn-state-body dn-state-title dn-state-text dn-state-last dn-btn dn-btn-secondary dn-btn-lg',
    ['role="alert"', 'Spróbuj ponownie', 'ostatnio OK 10:00']],
  ['Field input',
    () => h(DN.Field, { label: 'Nazwa zadania', grow: true, className: 'x', placeholder: 'p' }),
    'dn-field is-grow x dn-field-label dn-input',
    ['for="dn-f-nazwa-zadania"', 'id="dn-f-nazwa-zadania"', 'type="text"']],
  ['Field textarea', () => h(DN.Field, { label: 'Opis', multiline: true, id: 'opis' }), 'dn-field dn-field-label dn-input', ['<textarea', 'id="opis"']],
  ['Field select', () => h(DN.Field, { label: 'Typ', options: ['a', 'b'] }), 'dn-field dn-field-label dn-input', ['<select', '<option value="b">b</option>']],
  ['Flag new', () => h(DN.Flag, { tone: 'new' }, 'Nowe'), 'dn-flag dn-flag-new'],
  ['Flag info', () => h(DN.Flag, { tone: 'info', className: 'z' }, 'Zamknięte'), 'dn-flag dn-flag-info z'],
  ['Flag default', () => h(DN.Flag, null, 'Zmienione'), 'dn-flag dn-flag-change'],
  ['HoursBar',
    () => h(DN.HoursBar, { hours: 6.5, norm: 8, today: true, label: '6:30 na tle normy 8:00' }),
    'dn-hours dn-hours-fill is-today dn-hours-norm',
    ['width:65%', 'left:80%', 'role="img"']],
  ['HoursBar norm 0', () => h(DN.HoursBar, { hours: 2, norm: 0, label: 'x' }), 'dn-hours dn-hours-fill', [], ['dn-hours-norm']],
  ['Icon', () => h(DN.Icon, { name: 'hide', className: 'q', label: 'Ukryj' }), 'dn-icon q', ['role="img"', 'aria-label="Ukryj"', 'width="14"']],
  ['ItemRow checkbox',
    () => h(DN.ItemRow, {
      id: 'r1', title: 'Zadanie', source: 'gl', refId: '#27', flag: 'Nowe', flagTone: 'new', labels: ['a', 'b'],
      meta: '2 godz.', selected: true, done: true, onToggle: noop, checkboxLabel: 'Gotowe', checkboxDisabled: true,
    }, h('button', null, 'x')),
    'dn-row is-selected is-done dn-check dn-chip dn-chip-gl dn-row-ref dn-row-title dn-flag dn-flag-new dn-row-labels code dn-sr-only dn-row-meta dn-row-actions',
    ['type="checkbox"', 'id="r1"', 'for="r1"', 'aria-label="Gotowe"', 'disabled', 'checked', 'a, b']],
  ['ItemRow plain',
    () => h(DN.ItemRow, { title: 'Zadanie', flag: 'Zmienione', labels: [] }),
    'dn-row dn-row-title dn-flag dn-flag-change',
    ['<span class="dn-row-title">'], ['checkbox', 'dn-row-labels']],
  ['Notice error',
    () => h(DN.Notice, { tone: 'error', compact: true, action: 'Ponów', onAction: noop, className: 'x' }, 'Błąd'),
    'dn-notice dn-notice-error is-compact has-action x dn-icon dn-notice-icon dn-notice-body dn-notice-text dn-btn dn-btn-secondary dn-btn-sm dn-notice-action',
    ['role="alert"', 'width="14"']],
  ['Notice warn',
    () => h(DN.Notice, { tone: 'warn', title: 'Uwaga', inset: true, role: null }, 'tekst'),
    'dn-notice dn-notice-warn is-inset dn-icon dn-notice-icon dn-notice-body dn-notice-title dn-notice-text',
    ['width="16"'], ['role=']],
  ['Panel',
    () => h(DN.Panel, { title: 'Tytuł', meta: '[4/5]', footer: 'stopka', tone: 'danger', headingLevel: 1, className: 'x' }, 'treść'),
    'dn-panel is-danger x dn-panel-head dn-panel-title dn-panel-meta dn-panel-foot',
    ['<h1']],
  ['Segmented md',
    () => h(DN.Segmented, { options: [{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }], value: 'a', label: 'Widok', size: 'md' }),
    'dn-seg dn-seg-md dn-seg-btn is-on',
    ['role="group"', 'aria-pressed="true"', 'aria-pressed="false"']],
  ['SourceChip', () => h(DN.SourceChip, { source: 'mr', title: 't' }), 'dn-chip dn-chip-mr', ['>MR<']],
  ['SourceState calm',
    () => h(DN.SourceState, { kind: 'calm', title: 'Brak VPN', lastOk: 'ostatnio OK 9:00', action: 'Odśwież', onAction: noop, actionDisabled: true }, 'tekst'),
    'dn-state dn-state-calm dn-icon dn-state-icon dn-state-body dn-state-title dn-state-text dn-state-last dn-btn dn-btn-secondary dn-btn-lg',
    ['role="status"', 'aria-disabled="true"', 'aria-busy="true"', 'width="36"']],
  ['SourceState error',
    () => h(DN.SourceState, { kind: 'error', title: 'Błąd', action: 'Ponów', onAction: noop }),
    'dn-state dn-state-error dn-icon dn-state-icon dn-state-body dn-state-title dn-btn dn-btn-secondary dn-btn-lg',
    ['role="alert"', 'width="24"'], ['aria-disabled', 'aria-busy']],
  ['StatStrip',
    () => h(DN.StatStrip, { items: [{ label: 'Dziś', value: '6:30', sub: 'norma 8:00', tone: 'info' }, { label: 'Tydzień', value: '30:00' }] }),
    'dn-stats dn-stat dn-stat-label dn-stat-value is-info dn-stat-sub'],
  ['StatusPill', () => h(DN.StatusPill, { state: 'calm', className: 'x' }), 'dn-pill dn-pill-calm x', ['○', 'NIEDOSTĘPNY']],
  ['StatusPill text only', () => h(DN.StatusPill, { state: 'ok', glyph: false }, 'WŁ.'), 'dn-pill dn-pill-ok', ['WŁ.'], ['●']],
  ['Tabs',
    () => h(DN.Tabs, { tabs: [{ value: 'a', label: 'A', count: 3 }, { value: 'b', label: 'B' }], value: 'a', onChange: noop, label: 'Zakładki', panelId: 'pnl', idPrefix: 'tb' }),
    'dn-tabs dn-tab is-on dn-tab-count',
    ['role="tablist"', 'id="tb-a"', 'aria-controls="pnl"', 'tabindex="0"', 'tabindex="-1"', 'aria-selected="true"']],
];

const setOf = (s) => new Set(s.split(/\s+/).filter(Boolean));
if (DN && DN.Tabs) {
  for (const [name, build, expected, has = [], hasNot = []] of samples) {
    try {
      const html = renderToStaticMarkup(build());
      const got = new Set();
      for (const mm of html.matchAll(/class="([^"]*)"/g)) for (const c of mm[1].split(/\s+/)) if (c) got.add(c);
      const want = setOf(expected);
      const miss = [...want].filter((c) => !got.has(c));
      const extra = [...got].filter((c) => !want.has(c));
      const missText = has.filter((t) => !html.includes(t));
      const badText = hasNot.filter((t) => html.includes(t));
      const ok = !miss.length && !extra.length && !missText.length && !badText.length;
      report(ok, name, `missing classes [${miss}] extra classes [${extra}] missing text [${missText}] unexpected text [${badText}]`);
    } catch (e) {
      report(false, name, 'throws: ' + e.message);
    }
  }
}

console.log(failed ? `\n${failed} check(s) failed` : '\nbundle-check: all passed');
process.exit(failed ? 1 : 0);
