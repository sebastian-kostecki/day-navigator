/* @ds-bundle: {"format":4,"namespace":"DayNav","components":[{"name":"AppHeader"},{"name":"Button"},{"name":"Icon"},{"name":"SourceChip"},{"name":"Flag"},{"name":"StatusPill"},{"name":"StatStrip"},{"name":"Panel"},{"name":"ItemRow"},{"name":"Segmented"},{"name":"Tabs"},{"name":"SourceState"},{"name":"Field"},{"name":"HoursBar"}]} */
(function () {
  var React = window.React;
  var h = React.createElement;

  function cx() {
    return Array.prototype.filter.call(arguments, Boolean).join(' ');
  }
  function omit(obj, keys) {
    var out = {};
    for (var k in obj) if (Object.prototype.hasOwnProperty.call(obj, k) && keys.indexOf(k) < 0) out[k] = obj[k];
    return out;
  }

  var PATHS = {
    plus: ['M12 5v14M5 12h14'],
    close: ['M6 6l12 12M18 6L6 18'],
    refresh: ['M20 11a8 8 0 1 0-2.3 5.7', 'M20 4v7h-7'],
    edit: ['M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16z'],
    hide: ['M3 3l18 18', 'M10.6 5.1A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-3.2 4.1M6.6 6.6C3.8 8.4 2 12 2 12s3.5 7 10 7a9.7 9.7 0 0 0 5.4-1.6', 'M9.9 9.9a3 3 0 0 0 4.2 4.2'],
    show: ['M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z', 'M15 12a3 3 0 1 1-6 0a3 3 0 1 1 6 0'],
    external: ['M14 4h6v6', 'M20 4l-9 9', 'M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5'],
    'chevron-left': ['M15 6l-6 6 6 6'],
    'chevron-right': ['M9 6l6 6-6 6'],
    'wifi-off': ['M2 8.5a15 15 0 0 1 20 0', 'M5 12a10 10 0 0 1 9-2.6', 'M8.5 15.5a5 5 0 0 1 4.2-1.4', 'M12 19h.01', 'M3 3l18 18'],
    alert: ['M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z', 'M12 9v4M12 17h.01'],
    'check-circle': ['M21 12a9 9 0 1 1-18 0a9 9 0 1 1 18 0', 'M8 12l3 3 5-6'],
    repo: ['M4 4h12a4 4 0 0 1 4 4v12H8a4 4 0 0 1-4-4z', 'M4 16a4 4 0 0 1 4-4h12']
  };

  function Icon(props) {
    var size = props.size || 14;
    var d = PATHS[props.name] || PATHS.plus;
    var a11y = props.label ? { role: 'img', 'aria-label': props.label } : { 'aria-hidden': 'true' };
    return h('svg', Object.assign({
      width: size, height: size, viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor',
      strokeWidth: props.strokeWidth || 2, strokeLinecap: 'round', strokeLinejoin: 'round',
      className: cx('dn-icon', props.className)
    }, a11y), d.map(function (p, i) { return h('path', { key: i, d: p }); }));
  }

  function Button(props) {
    var variant = props.variant || 'secondary';
    var size = props.size || 'md';
    var rest = omit(props, ['variant', 'size', 'icon', 'iconOnly', 'pressed', 'className', 'children']);
    var extra = {};
    if (props.pressed !== undefined) extra['aria-pressed'] = props.pressed ? 'true' : 'false';
    return h('button', Object.assign({ type: 'button' }, rest, extra, {
      className: cx('dn-btn', 'dn-btn-' + variant, 'dn-btn-' + size, props.iconOnly && 'dn-btn-icon', props.pressed && 'is-pressed', props.className)
    }),
      props.icon ? h(Icon, { name: props.icon }) : null,
      props.iconOnly ? null : props.children);
  }

  function AppHeader(props) {
    var items = props.items || [];
    var running = props.running !== false;
    return h('header', { className: 'dn-header' },
      h('div', { className: 'dn-brand' }, props.brand || 'DAY/NAV'),
      h('nav', { className: 'dn-nav', 'aria-label': 'Widoki' }, items.map(function (it, i) {
        return h('a', {
          key: it.key || i, href: it.href || '#', className: cx('dn-nav-item', it.active && 'is-active'),
          'aria-current': it.active ? 'page' : undefined, onClick: it.onClick
        }, h('span', { className: 'dn-nav-key' }, it.key), it.label);
      })),
      h('div', { className: 'dn-timer' },
        h('span', { className: cx('dn-rec', running && 'is-on') }, running ? '● REC' : '○ STOP'),
        h('span', { className: 'dn-clock' }, props.clock || '00:00:00'),
        h(Button, { variant: running ? 'secondary' : 'primary', size: 'lg', className: 'dn-btn-mono', onClick: props.onToggle }, running ? 'STOP' : 'START')));
  }

  var CHIP = { gl: 'GL', gh: 'GH', my: 'MY', mr: 'MR', iss: 'ISS' };
  function SourceChip(props) {
    var s = props.source || 'gl';
    return h('span', { className: cx('dn-chip', 'dn-chip-' + s, props.className), title: props.title }, props.children || CHIP[s] || s);
  }

  function Flag(props) {
    return h('span', { className: cx('dn-flag', 'dn-flag-' + (props.tone || 'change'), props.className) }, props.children);
  }

  var PILL = { ok: ['●', 'OK'], calm: ['○', 'NIEDOSTĘPNY'], error: ['▲', 'BŁĄD'] };
  function StatusPill(props) {
    var st = props.state || 'ok';
    var p = PILL[st] || PILL.ok;
    return h('span', { className: cx('dn-pill', 'dn-pill-' + st, props.className) },
      props.glyph === false ? null : h('span', { 'aria-hidden': 'true' }, p[0]),
      props.children || p[1]);
  }

  function StatStrip(props) {
    return h('div', { className: 'dn-stats' }, (props.items || []).map(function (s, i) {
      return h('div', { key: i, className: 'dn-stat' },
        h('span', { className: 'dn-stat-label' }, s.label),
        h('span', { className: cx('dn-stat-value', s.tone && 'is-' + s.tone) }, s.value),
        s.sub ? h('span', { className: 'dn-stat-sub' }, s.sub) : null);
    }));
  }

  function Panel(props) {
    var Tag = props.headingLevel === 1 ? 'h1' : 'h2';
    return h('section', { className: cx('dn-panel', props.tone === 'danger' && 'is-danger', props.className), style: props.style },
      props.title ? h('div', { className: 'dn-panel-head' },
        h(Tag, { className: 'dn-panel-title' }, props.title),
        props.meta ? h('span', { className: 'dn-panel-meta' }, props.meta) : null) : null,
      props.children,
      props.footer ? h('div', { className: 'dn-panel-foot' }, props.footer) : null);
  }

  function ItemRow(props) {
    var id = props.id || ('dn-item-' + Math.random().toString(36).slice(2, 8));
    return h('div', { className: cx('dn-row', props.selected && 'is-selected', props.done && 'is-done') },
      props.onToggle !== undefined || props.done !== undefined
        ? h('input', { type: 'checkbox', id: id, className: 'dn-check', checked: !!props.done, onChange: props.onToggle || function () {} })
        : null,
      props.source ? h(SourceChip, { source: props.source }) : null,
      props.refId ? h('span', { className: 'dn-row-ref' }, props.refId) : null,
      h('label', { htmlFor: id, className: 'dn-row-title' }, props.title),
      props.flag ? h(Flag, { tone: props.flagTone || 'change' }, props.flag) : null,
      props.meta ? h('span', { className: 'dn-row-meta' }, props.meta) : null,
      props.children ? h('div', { className: 'dn-row-actions' }, props.children) : null);
  }

  function Segmented(props) {
    return h('div', { role: 'group', 'aria-label': props.label, className: 'dn-seg' }, (props.options || []).map(function (o) {
      var on = o.value === props.value;
      return h('button', {
        key: o.value, type: 'button', 'aria-pressed': on ? 'true' : 'false', className: cx('dn-seg-btn', on && 'is-on'),
        onClick: function () { if (props.onChange) props.onChange(o.value); }
      }, o.label);
    }));
  }

  function Tabs(props) {
    return h('div', { role: 'tablist', 'aria-label': props.label, className: 'dn-tabs' }, (props.tabs || []).map(function (t) {
      var on = t.value === props.value;
      return h('button', {
        key: t.value, type: 'button', role: 'tab', 'aria-selected': on ? 'true' : 'false', className: cx('dn-tab', on && 'is-on'),
        onClick: function () { if (props.onChange) props.onChange(t.value); }
      }, t.label, t.count !== undefined ? h('span', { className: 'dn-tab-count' }, t.count) : null);
    }));
  }

  var STATE_ICON = { calm: 'wifi-off', error: 'alert', empty: 'check-circle' };
  function SourceState(props) {
    var kind = props.kind || 'empty';
    return h('section', { className: cx('dn-state', 'dn-state-' + kind), role: kind === 'error' ? 'alert' : undefined },
      h(Icon, { name: STATE_ICON[kind], size: kind === 'error' ? 24 : 36, strokeWidth: kind === 'error' ? 2 : 1.5, className: 'dn-state-icon' }),
      h('div', { className: 'dn-state-body' },
        h('h2', { className: 'dn-state-title' }, props.title),
        props.children ? h('p', { className: 'dn-state-text' }, props.children) : null,
        props.lastOk ? h('p', { className: 'dn-state-last' }, 'ostatnie udane odświeżenie: ' + props.lastOk) : null,
        props.action ? h(Button, { variant: 'secondary', size: 'lg', onClick: props.onAction }, props.action) : null));
  }

  function Field(props) {
    var id = props.id || ('dn-f-' + Math.random().toString(36).slice(2, 8));
    var rest = omit(props, ['label', 'id', 'options', 'className', 'grow']);
    var control = props.options
      ? h('select', Object.assign({ id: id, className: 'dn-input' }, rest), props.options.map(function (o) { return h('option', { key: o, value: o }, o); }))
      : h('input', Object.assign({ id: id, type: 'text', className: 'dn-input' }, rest));
    return h('div', { className: cx('dn-field', props.grow && 'is-grow', props.className) },
      h('label', { htmlFor: id, className: 'dn-field-label' }, props.label), control);
  }

  function HoursBar(props) {
    var max = props.max || 10;
    var norm = props.norm === undefined ? 8 : props.norm;
    var pct = Math.max(0, Math.min(100, ((props.hours || 0) / max) * 100));
    return h('div', { className: 'dn-hours', role: 'img', 'aria-label': props.label || ((props.hours || 0) + ' h na tle normy ' + norm + ' h') },
      h('div', { className: cx('dn-hours-fill', props.today && 'is-today'), style: { width: pct + '%' } }),
      norm > 0 ? h('div', { className: 'dn-hours-norm', style: { left: (norm / max) * 100 + '%' } }) : null);
  }

  window.DayNav = Object.assign(window.DayNav || {}, {
    AppHeader: AppHeader, Button: Button, Icon: Icon, SourceChip: SourceChip, Flag: Flag, StatusPill: StatusPill,
    StatStrip: StatStrip, Panel: Panel, ItemRow: ItemRow, Segmented: Segmented, Tabs: Tabs, SourceState: SourceState,
    Field: Field, HoursBar: HoursBar
  });
})();
