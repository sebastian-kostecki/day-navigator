/* @ds-bundle: {"format":4,"namespace":"DayNav","components":[{"name":"AppHeader"},{"name":"Button"},{"name":"Chip"},{"name":"DataTable"},{"name":"ErrorState"},{"name":"Field"},{"name":"Flag"},{"name":"HoursBar"},{"name":"Icon"},{"name":"ItemRow"},{"name":"Notice"},{"name":"Panel"},{"name":"Segmented"},{"name":"SourceChip"},{"name":"SourceState"},{"name":"StatStrip"},{"name":"StatusPill"},{"name":"Tabs"}]} */
/* Port of src/components/** (React 19 TSX) to a classic script on React 18 globals. */
(function () {
  var React = window.React;
  var h = React.createElement;

  function cx() {
    return Array.prototype.filter.call(arguments, Boolean).join(' ');
  }
  function omit(obj, keys) {
    var out = {};
    for (var k in obj) {
      if (Object.prototype.hasOwnProperty.call(obj, k) && keys.indexOf(k) < 0) out[k] = obj[k];
    }
    return out;
  }
  function assign(target) {
    for (var i = 1; i < arguments.length; i++) {
      var src = arguments[i];
      for (var k in src) if (Object.prototype.hasOwnProperty.call(src, k)) target[k] = src[k];
    }
    return target;
  }

  /* ---------- Icon ---------- */
  var EYE = [
    'M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z',
    'M15 12a3 3 0 1 1-6 0a3 3 0 1 1 6 0'
  ];
  var EYE_OFF = [
    'M3 3l18 18',
    'M10.6 5.1A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-3.2 4.1M6.6 6.6C3.8 8.4 2 12 2 12s3.5 7 10 7a9.7 9.7 0 0 0 5.4-1.6',
    'M9.9 9.9a3 3 0 0 0 4.2 4.2'
  ];
  var ALERT_TRIANGLE = [
    'M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z',
    'M12 9v4M12 17h.01'
  ];
  var PATHS = {
    plus: ['M12 5v14M5 12h14'],
    close: ['M6 6l12 12M18 6L6 18'],
    refresh: ['M20 11a8 8 0 1 0-2.3 5.7', 'M20 4v7h-7'],
    edit: ['M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16z'],
    eye: EYE,
    'eye-off': EYE_OFF,
    external: ['M14 4h6v6', 'M20 4l-9 9', 'M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5'],
    'chevron-left': ['M15 6l-6 6 6 6'],
    'chevron-right': ['M9 6l6 6-6 6'],
    'wifi-off': [
      'M2 8.5a15 15 0 0 1 20 0',
      'M5 12a10 10 0 0 1 9-2.6',
      'M8.5 15.5a5 5 0 0 1 4.2-1.4',
      'M12 19h.01',
      'M3 3l18 18'
    ],
    'alert-triangle': ALERT_TRIANGLE,
    'check-circle': ['M21 12a9 9 0 1 1-18 0a9 9 0 1 1 18 0', 'M8 12l3 3 5-6'],
    repo: ['M4 4h12a4 4 0 0 1 4 4v12H8a4 4 0 0 1-4-4z', 'M4 16a4 4 0 0 1 4-4h12'],
    clock: ['M21 12a9 9 0 1 1-18 0a9 9 0 1 1 18 0', 'M12 7v5l3 2'],
    folder: ['M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z'],
    hide: EYE_OFF,
    show: EYE,
    alert: ALERT_TRIANGLE
  };

  function Icon(props) {
    var size = props.size === undefined ? 14 : props.size;
    var strokeWidth = props.strokeWidth === undefined ? 2 : props.strokeWidth;
    var a11y = props.label
      ? { role: 'img', 'aria-label': props.label }
      : { 'aria-hidden': true };
    return h(
      'svg',
      assign(
        {
          width: size,
          height: size,
          viewBox: '0 0 24 24',
          fill: 'none',
          stroke: 'currentColor',
          strokeWidth: strokeWidth,
          strokeLinecap: 'round',
          strokeLinejoin: 'round',
          className: props.className ? 'dn-icon ' + props.className : 'dn-icon'
        },
        a11y
      ),
      PATHS[props.name].map(function (d) {
        return h('path', { key: d, d: d });
      })
    );
  }

  /* ---------- Button ---------- */
  var Button = React.forwardRef(function Button(props, ref) {
    var variant = props.variant === undefined ? 'secondary' : props.variant;
    var size = props.size === undefined ? 'md' : props.size;
    var type = props.type === undefined ? 'button' : props.type;
    var pressed = props.pressed;
    var rest = omit(props, ['variant', 'size', 'icon', 'iconOnly', 'pressed', 'className', 'children', 'type']);
    var classes = cx(
      'dn-btn',
      'dn-btn-' + variant,
      'dn-btn-' + size,
      props.iconOnly && 'dn-btn-icon',
      pressed && 'is-pressed',
      props.className
    );
    return h(
      'button',
      assign({ type: type, 'aria-pressed': pressed, ref: ref }, rest, { className: classes }),
      props.icon ? h(Icon, { name: props.icon }) : null,
      props.iconOnly ? null : props.children
    );
  });

  /* ---------- AppHeader ---------- */
  function AppHeader(props) {
    var items = props.items || [];
    return h(
      'header',
      { className: 'dn-header' },
      h('span', { className: 'dn-brand' }, 'DAY/NAV'),
      h(
        'nav',
        { className: 'dn-nav', 'aria-label': 'Widoki' },
        items.map(function (item) {
          return h(
            'button',
            {
              key: item.id,
              type: 'button',
              className: 'dn-nav-item' + (item.active ? ' is-active' : ''),
              'aria-current': item.active ? 'page' : undefined,
              'aria-keyshortcuts': item.key,
              'data-view': item.id,
              onClick: function () {
                if (props.onSelect) props.onSelect(item.id);
              }
            },
            item.key ? h('span', { className: 'dn-nav-key', 'aria-hidden': 'true' }, item.key) : null,
            item.label
          );
        })
      ),
      props.children
    );
  }

  /* ---------- Chip ---------- */
  function Chip(props) {
    return h(
      'span',
      { className: cx('dn-chip', props.tone && 'dn-chip-' + props.tone, props.className) },
      props.children
    );
  }

  /* ---------- DataTable ---------- */
  function DataTable(props) {
    return h(
      'div',
      { role: 'table', 'aria-label': props.label, className: 'dn-table ' + props.className },
      props.children
    );
  }
  function DataTableRow(props) {
    return h(
      'div',
      {
        role: 'row',
        'aria-current': props.current ? 'true' : undefined,
        className: cx(
          'dn-table-row',
          props.head ? 'dn-table-head label-caps' : 'dn-table-body',
          props.current && 'is-current',
          props.hatched && 'is-hatched',
          props.className
        )
      },
      props.children
    );
  }
  function DataTableCell(props) {
    return h(
      'span',
      {
        role: props.header ? 'columnheader' : 'cell',
        title: props.title,
        className: cx(
          'dn-table-cell',
          props.tone && 'dn-table-cell-' + props.tone,
          props.truncate && 'dn-table-cell-truncate',
          props.className
        )
      },
      props.children
    );
  }

  /* ---------- SourceState / ErrorState ---------- */
  var STATE_ICON = { empty: 'check-circle', calm: 'wifi-off', error: 'alert-triangle' };

  function SourceState(props) {
    var kind = props.kind;
    var role = kind === 'error' ? 'alert' : kind === 'calm' ? 'status' : undefined;
    var actionDisabled = props.actionDisabled;
    return h(
      'section',
      { className: 'dn-state dn-state-' + kind, role: role },
      h(Icon, {
        name: STATE_ICON[kind],
        size: kind === 'error' ? 24 : 36,
        strokeWidth: kind === 'error' ? 2 : 1.5,
        className: 'dn-state-icon'
      }),
      h(
        'div',
        { className: 'dn-state-body' },
        h('h2', { className: 'dn-state-title' }, props.title),
        props.children ? h('p', { className: 'dn-state-text' }, props.children) : null,
        props.lastOk ? h('p', { className: 'dn-state-last' }, props.lastOk) : null,
        props.action
          ? h(
              Button,
              {
                variant: 'secondary',
                size: 'lg',
                onClick: actionDisabled ? undefined : props.onAction,
                'aria-disabled': actionDisabled || undefined,
                'aria-busy': actionDisabled || undefined
              },
              props.action
            )
          : null
      )
    );
  }

  function ErrorState(props) {
    return h(
      SourceState,
      {
        kind: 'error',
        title: props.title,
        lastOk: props.lastSuccess,
        action: props.onRetry ? 'Spróbuj ponownie' : undefined,
        onAction: props.onRetry
      },
      props.message
    );
  }

  /* ---------- Field ---------- */
  function slug(text) {
    return String(text)
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
  }

  function Field(props) {
    var id = props.id !== undefined ? props.id : 'dn-f-' + slug(props.label);
    var classes = cx('dn-field', props.grow && 'is-grow', props.className);
    var rest = omit(props, ['label', 'grow', 'id', 'className', 'multiline', 'options']);
    var control;
    if (props.multiline === true) {
      control = h('textarea', assign({ id: id, className: 'dn-input' }, rest));
    } else if (props.options !== undefined) {
      control = h(
        'select',
        assign({ id: id, className: 'dn-input' }, rest),
        props.options.map(function (o) {
          return h('option', { key: o, value: o }, o);
        })
      );
    } else {
      control = h('input', assign({ id: id, type: 'text', className: 'dn-input' }, rest));
    }
    return h(
      'div',
      { className: classes },
      h('label', { htmlFor: id, className: 'dn-field-label' }, props.label),
      control
    );
  }

  /* ---------- Flag ---------- */
  function Flag(props) {
    var tone = props.tone === undefined ? 'change' : props.tone;
    return h('span', { className: cx('dn-flag', 'dn-flag-' + tone, props.className) }, props.children);
  }

  /* ---------- HoursBar ---------- */
  function HoursBar(props) {
    var max = props.max === undefined ? 10 : props.max;
    var pct = Math.max(0, Math.min(100, (props.hours / max) * 100));
    return h(
      'div',
      { className: 'dn-hours', role: 'img', 'aria-label': props.label },
      h('div', {
        className: props.today ? 'dn-hours-fill is-today' : 'dn-hours-fill',
        style: { width: pct + '%' }
      }),
      props.norm > 0
        ? h('div', { className: 'dn-hours-norm', style: { left: (props.norm / max) * 100 + '%' } })
        : null
    );
  }

  /* ---------- SourceChip ---------- */
  var SOURCE_LABELS = { gl: 'GL', gh: 'GH', my: 'MY', mr: 'MR', iss: 'ISS' };
  function SourceChip(props) {
    return h(
      'span',
      { className: cx('dn-chip', 'dn-chip-' + props.source, props.className), title: props.title },
      props.children !== undefined && props.children !== null ? props.children : SOURCE_LABELS[props.source]
    );
  }

  /* ---------- ItemRow ---------- */
  function ItemRow(props) {
    var labels = props.labels;
    var joinedLabels = labels && labels.length > 0 ? labels.join(', ') : null;
    var done = props.done === true || props.checked === true;
    var classes = cx('dn-row', props.selected && 'is-selected', done && 'is-done');
    return h(
      'div',
      { className: classes },
      props.onToggle
        ? h('input', {
            type: 'checkbox',
            id: props.id,
            className: 'dn-check',
            checked: done,
            disabled: props.checkboxDisabled,
            'aria-label': props.checkboxLabel,
            onChange: props.onToggle
          })
        : null,
      props.source ? h(SourceChip, { source: props.source }) : null,
      props.refId ? h('span', { className: 'dn-row-ref' }, props.refId) : null,
      props.onToggle
        ? h('label', { htmlFor: props.id, className: 'dn-row-title' }, props.title)
        : h('span', { className: 'dn-row-title' }, props.title),
      props.flag ? h(Flag, { tone: props.flagTone }, props.flag) : null,
      joinedLabels
        ? h(
            'span',
            { className: 'dn-row-labels code', title: joinedLabels },
            h('span', { className: 'dn-sr-only' }, 'Etykiety: '),
            joinedLabels
          )
        : null,
      props.meta ? h('span', { className: 'dn-row-meta' }, props.meta) : null,
      props.children ? h('div', { className: 'dn-row-actions' }, props.children) : null
    );
  }

  /* ---------- Notice ---------- */
  var TONE_ICON = {
    calm: 'wifi-off',
    info: 'check-circle',
    warn: 'alert-triangle',
    error: 'alert-triangle'
  };
  function Notice(props) {
    var tone = props.tone;
    var role = props.role;
    var resolvedRole = role === undefined ? (tone === 'error' ? 'alert' : 'status') : role;
    var classes = cx(
      'dn-notice',
      'dn-notice-' + tone,
      props.inset && 'is-inset',
      props.compact && 'is-compact',
      props.action && 'has-action',
      props.className
    );
    return h(
      'div',
      { className: classes, role: resolvedRole === null ? undefined : resolvedRole },
      h(Icon, { name: TONE_ICON[tone], size: props.compact ? 14 : 16, className: 'dn-notice-icon' }),
      h(
        'div',
        { className: 'dn-notice-body' },
        props.title ? h('p', { className: 'dn-notice-title' }, props.title) : null,
        h('div', { className: 'dn-notice-text' }, props.children)
      ),
      props.action
        ? h(
            Button,
            {
              variant: 'secondary',
              size: 'sm',
              className: 'dn-notice-action',
              disabled: props.actionDisabled,
              onClick: props.onAction
            },
            props.action
          )
        : null
    );
  }

  /* ---------- Panel ---------- */
  function Panel(props) {
    var tone = props.tone === undefined ? 'default' : props.tone;
    var headingLevel = props.headingLevel === undefined ? 2 : props.headingLevel;
    var Heading = headingLevel === 1 ? 'h1' : 'h2';
    var classes = cx('dn-panel', tone === 'danger' && 'is-danger', props.className);
    return h(
      'section',
      { className: classes },
      props.title
        ? h(
            'div',
            { className: 'dn-panel-head' },
            h(Heading, { className: 'dn-panel-title' }, props.title),
            props.meta ? h('span', { className: 'dn-panel-meta' }, props.meta) : null
          )
        : null,
      props.children,
      props.footer ? h('div', { className: 'dn-panel-foot' }, props.footer) : null
    );
  }

  /* ---------- Segmented ---------- */
  function Segmented(props) {
    var size = props.size === undefined ? 'sm' : props.size;
    return h(
      'div',
      { role: 'group', 'aria-label': props.label, className: size === 'md' ? 'dn-seg dn-seg-md' : 'dn-seg' },
      props.options.map(function (o) {
        var on = o.value === props.value;
        return h(
          'button',
          {
            key: o.value,
            type: 'button',
            'aria-pressed': on,
            className: on ? 'dn-seg-btn is-on' : 'dn-seg-btn',
            onClick: function () {
              if (props.onChange) props.onChange(o.value);
            }
          },
          o.label
        );
      })
    );
  }

  /* ---------- StatStrip ---------- */
  function StatStrip(props) {
    return h(
      'div',
      { className: 'dn-stats' },
      props.items.map(function (s) {
        return h(
          'div',
          { key: s.label, className: 'dn-stat' },
          h('span', { className: 'dn-stat-label' }, s.label),
          h('span', { className: s.tone ? 'dn-stat-value is-' + s.tone : 'dn-stat-value' }, s.value),
          s.sub ? h('span', { className: 'dn-stat-sub' }, s.sub) : null
        );
      })
    );
  }

  /* ---------- StatusPill ---------- */
  var PILL = {
    ok: { glyph: '●', text: 'OK' },
    calm: { glyph: '○', text: 'NIEDOSTĘPNY' },
    error: { glyph: '▲', text: 'BŁĄD' }
  };
  function StatusPill(props) {
    var p = PILL[props.state];
    var glyph = props.glyph === undefined ? true : props.glyph;
    return h(
      'span',
      { className: cx('dn-pill', 'dn-pill-' + props.state, props.className) },
      glyph ? h('span', { 'aria-hidden': 'true' }, p.glyph) : null,
      props.children !== undefined && props.children !== null ? props.children : p.text
    );
  }

  /* ---------- Tabs ---------- */
  function Tabs(props) {
    var tabs = props.tabs;
    var refs = React.useRef([]);

    function onKeyDown(e, index) {
      var next;
      if (e.key === 'ArrowRight') next = (index + 1) % tabs.length;
      else if (e.key === 'ArrowLeft') next = (index - 1 + tabs.length) % tabs.length;
      else if (e.key === 'Home') next = 0;
      else if (e.key === 'End') next = tabs.length - 1;
      else return;
      e.preventDefault();
      if (props.onChange) props.onChange(tabs[next].value);
      if (refs.current[next]) refs.current[next].focus();
    }

    return h(
      'div',
      { role: 'tablist', 'aria-label': props.label, className: 'dn-tabs' },
      tabs.map(function (t, i) {
        var on = t.value === props.value;
        return h(
          'button',
          {
            key: t.value,
            ref: function (el) {
              refs.current[i] = el;
            },
            type: 'button',
            role: 'tab',
            id: props.idPrefix + '-' + t.value,
            'aria-selected': on,
            'aria-controls': props.panelId,
            tabIndex: on ? 0 : -1,
            className: on ? 'dn-tab is-on' : 'dn-tab',
            onClick: function () {
              if (props.onChange) props.onChange(t.value);
            },
            onKeyDown: function (e) {
              onKeyDown(e, i);
            }
          },
          t.label,
          t.count !== undefined && h('span', { className: 'dn-tab-count' }, t.count)
        );
      })
    );
  }

  window.DayNav = {
    AppHeader: AppHeader,
    Button: Button,
    Chip: Chip,
    DataTable: DataTable,
    DataTableCell: DataTableCell,
    DataTableRow: DataTableRow,
    ErrorState: ErrorState,
    Field: Field,
    Flag: Flag,
    HoursBar: HoursBar,
    Icon: Icon,
    ItemRow: ItemRow,
    Notice: Notice,
    Panel: Panel,
    Segmented: Segmented,
    SourceChip: SourceChip,
    SourceState: SourceState,
    StatStrip: StatStrip,
    StatusPill: StatusPill,
    Tabs: Tabs
  };
})();
