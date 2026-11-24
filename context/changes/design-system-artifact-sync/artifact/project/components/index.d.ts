// Types as documentation: they mirror src/components/** (React 19 TSX). Nothing here is executed.
import type {
  ButtonHTMLAttributes,
  InputHTMLAttributes,
  ReactNode,
  Ref,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from 'react';

export type IconName =
  | 'plus'
  | 'close'
  | 'refresh'
  | 'edit'
  | 'eye'
  | 'eye-off'
  | 'external'
  | 'chevron-left'
  | 'chevron-right'
  | 'wifi-off'
  | 'alert-triangle'
  | 'check-circle'
  | 'repo'
  | 'clock'
  | 'folder'
  /** Older aliases: hide = eye-off, show = eye, alert = alert-triangle. */
  | 'hide'
  | 'show'
  | 'alert';

export type IconProps = {
  name: IconName;
  /** Pixel size, default 14. */
  size?: number;
  /** Accessible name; without it the icon is decorative (aria-hidden). */
  label?: string;
  strokeWidth?: number;
  className?: string;
};

export type ButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'> & {
  /** primary = one per view (accent fill); secondary = default; quiet = in-row tools; dashed = "add missing". */
  variant?: 'primary' | 'secondary' | 'quiet' | 'dashed';
  /** sm 36px, md 40px, lg 44px. */
  size?: 'sm' | 'md' | 'lg';
  icon?: IconName;
  /** Icon-only square button: pass aria-label. */
  iconOnly?: boolean;
  /** Toggle state, rendered as aria-pressed. */
  pressed?: boolean;
  /** Ref as a prop, e.g. to move focus to the button (the preview bundle forwards it with React.forwardRef). */
  ref?: Ref<HTMLButtonElement>;
  children?: ReactNode;
};

export type AppHeaderProps = {
  /** key: the shortcut shown before the label ("1"…"6"), also set as aria-keyshortcuts. */
  items: { id: string; label: string; active: boolean; key?: string }[];
  onSelect: (id: string) => void;
  /** Right-hand slot (the work-time counter). */
  children?: ReactNode;
};

export type ChipProps = {
  /** warn = amber text, soft = soft ink, faint = faint ink. Omitted: inherited colour. */
  tone?: 'warn' | 'soft' | 'faint';
  children: ReactNode;
  className?: string;
};

export type DataTableProps = {
  /** Accessible name of the table. */
  label: string;
  /**
   * Required: the view sets `--dn-table-cols` (the grid column template) on this
   * class, and optionally `--dn-table-pad-x`; without it the rows collapse.
   */
  className: string;
  children: ReactNode;
};

export type DataTableRowProps = {
  /** Header row: caps labels, control height, stronger bottom line. */
  head?: boolean;
  /** The row that stands for now: raised background, accent edge, `aria-current`. */
  current?: boolean;
  /** Diagonal stripes (a day off); combines with `current`. */
  hatched?: boolean;
  className?: string;
  children: ReactNode;
};

export type DataTableCellProps = {
  /** Column header cell (role=columnheader) instead of a data cell. */
  header?: boolean;
  /** muted = secondary; faint = meta text; info / warn = a figure above / below its norm. */
  tone?: 'muted' | 'faint' | 'info' | 'warn';
  /** One line with an ellipsis. */
  truncate?: boolean;
  className?: string;
  title?: string;
  children?: ReactNode;
};

export type ErrorStateProps = {
  title: string;
  /** Inline content only (text, `<code>`, links): it is rendered inside a `<p>`. */
  message: ReactNode;
  onRetry?: () => void;
  /** Time of the last successful refresh, shown under the message as given. */
  lastSuccess?: string;
};

type FieldBase = {
  /** Always visible; linked to the control through htmlFor. */
  label: string;
  /** Takes the free width in a wrapping form row. */
  grow?: boolean;
  /** Without it the preview bundle derives "dn-f-<label slug>" (the app uses useId). */
  id?: string;
  className?: string;
};

type InputField = FieldBase &
  Omit<InputHTMLAttributes<HTMLInputElement>, 'id' | 'className'> & {
    multiline?: false;
    options?: undefined;
  };

type TextareaField = FieldBase &
  Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, 'id' | 'className'> & {
    /** Renders a textarea. */
    multiline: true;
    options?: undefined;
  };

type SelectField = FieldBase &
  Omit<SelectHTMLAttributes<HTMLSelectElement>, 'id' | 'className'> & {
    /** Renders a select with these options. */
    options: string[];
    multiline?: false;
  };

export type FieldProps = InputField | TextareaField | SelectField;

export type FlagProps = {
  /** change = amber (Zmienione, Z <dnia>, Draft); info = blue (closed in the source);
   * new = blue (Nowe, a freshly assigned item). */
  tone?: 'change' | 'info' | 'new';
  children: ReactNode;
  className?: string;
};

export type HoursBarProps = {
  /** Worked hours. */
  hours: number;
  /** The day's norm in hours, from the backend; `0` hides the norm tick. */
  norm: number;
  /** Scale of the bar in hours. */
  max?: number;
  /** Accent fill for the current day. */
  today?: boolean;
  /** Accessible text, e.g. "6:30 na tle normy 8:00". */
  label: string;
};

export type SourceKind = 'gl' | 'gh' | 'my' | 'mr' | 'iss';

export type SourceChipProps = {
  /** gl GitLab, gh GitHub, my own task, mr merge request, iss issue. */
  source: SourceKind;
  children?: ReactNode;
  title?: string;
  className?: string;
};

type RowBase = {
  title: ReactNode;
  /** Source chip before the ref; no chip when absent. */
  source?: SourceKind;
  /** Issue / MR number: "!418", "#27". */
  refId?: string;
  flag?: ReactNode;
  flagTone?: FlagProps['tone'];
  /** Label names, shown comma-separated; nothing is rendered when absent or empty. */
  labels?: string[];
  /** Right-aligned mono meta: due date, "2 godz.". */
  meta?: ReactNode;
  selected?: boolean;
  /** Row actions: quiet icon Buttons. */
  children?: ReactNode;
};

type WithCheckbox = RowBase & {
  /** Required with a checkbox: links the label to it. */
  id: string;
  /** Ticked: checkbox checked, title struck through and dimmed. */
  done?: boolean;
  /** Same as `done` (either one ticks the row). */
  checked?: boolean;
  onToggle: () => void;
  /** Accessible name of the checkbox. */
  checkboxLabel?: string;
  checkboxDisabled?: boolean;
};

type WithoutCheckbox = RowBase & {
  id?: string;
  done?: undefined;
  checked?: undefined;
  onToggle?: undefined;
  checkboxLabel?: undefined;
  checkboxDisabled?: undefined;
};

export type ItemRowProps = WithCheckbox | WithoutCheckbox;

export type NoticeTone = 'calm' | 'info' | 'warn' | 'error';

export type NoticeProps = {
  /** calm = unavailable, never red; info = done / FYI; warn = degraded or misconfigured; error = an action or part failed. */
  tone: NoticeTone;
  children: ReactNode;
  /** Optional bold line above the text. */
  title?: string;
  /** Label of the small secondary button on the right. */
  action?: string;
  onAction?: () => void;
  actionDisabled?: boolean;
  /** Inside a Panel body: keeps the panel's side padding around the banner. */
  inset?: boolean;
  /** Smaller type and padding: under a form field, in a row, in the header. */
  compact?: boolean;
  /** Defaults to alert for error, status otherwise; null when an outer live region announces it. */
  role?: 'alert' | 'status' | null;
  className?: string;
};

export type PanelProps = {
  title?: ReactNode;
  /** Right side of the head, mono: "[4/5]", "15 pozycji". */
  meta?: ReactNode;
  footer?: ReactNode;
  tone?: 'default' | 'danger';
  /** Heading element of the title; h2 by default. */
  headingLevel?: 1 | 2;
  className?: string;
  children?: ReactNode;
};

export type SegmentedOption = { value: string; label: string };

export type SegmentedProps = {
  options: SegmentedOption[];
  value: string;
  onChange?: (value: string) => void;
  /** Accessible name of the group. */
  label: string;
  /** sm = control height (default); md = matches the 40px fields. */
  size?: 'sm' | 'md';
};

export type SourceStateProps = {
  /** empty = data with nothing in it; calm = unavailable, no alarm (role=status); error = real failure (role=alert). */
  kind: 'empty' | 'calm' | 'error';
  title: string;
  children?: ReactNode;
  /** Ready text about the last successful refresh, rendered as given (no prefix). */
  lastOk?: string;
  /** Label of the secondary action button. */
  action?: string;
  onAction?: () => void;
  /**
   * Blocks the action while clicking it would change nothing (e.g. a refresh is
   * running). It stays focusable (aria-disabled), so keyboard focus is not lost.
   */
  actionDisabled?: boolean;
};

export type Stat = {
  label: string;
  value: string;
  sub?: string;
  /** Value colour: info (blue, above norm / OK), warn (orange, below norm), soft (ink-soft); muted = soft. */
  tone?: 'info' | 'warn' | 'soft' | 'muted';
};

export type StatStripProps = { items: Stat[] };

export type PillState = 'ok' | 'calm' | 'error';

export type StatusPillProps = {
  /** ok ●, calm ○ (no VPN), error ▲. */
  state: PillState;
  /** Defaults to OK / NIEDOSTĘPNY / BŁĄD. */
  children?: ReactNode;
  /** Show the ●/○/▲ glyph before the text; false renders the text only. Default true. */
  glyph?: boolean;
  className?: string;
};

export type TabsProps<T extends string> = {
  tabs: { value: T; label: string; count?: number }[];
  value: T;
  onChange: (value: T) => void;
  /** Accessible name of the tab group. */
  label: string;
  /** Id of the tab panel these tabs control. */
  panelId: string;
  idPrefix: string;
};

declare const DayNav: {
  AppHeader: (props: AppHeaderProps) => ReactNode;
  Button: (props: ButtonProps) => ReactNode;
  Chip: (props: ChipProps) => ReactNode;
  DataTable: (props: DataTableProps) => ReactNode;
  DataTableCell: (props: DataTableCellProps) => ReactNode;
  DataTableRow: (props: DataTableRowProps) => ReactNode;
  ErrorState: (props: ErrorStateProps) => ReactNode;
  Field: (props: FieldProps) => ReactNode;
  Flag: (props: FlagProps) => ReactNode;
  HoursBar: (props: HoursBarProps) => ReactNode;
  Icon: (props: IconProps) => ReactNode;
  ItemRow: (props: ItemRowProps) => ReactNode;
  Notice: (props: NoticeProps) => ReactNode;
  Panel: (props: PanelProps) => ReactNode;
  Segmented: (props: SegmentedProps) => ReactNode;
  SourceChip: (props: SourceChipProps) => ReactNode;
  SourceState: (props: SourceStateProps) => ReactNode;
  StatStrip: (props: StatStripProps) => ReactNode;
  StatusPill: (props: StatusPillProps) => ReactNode;
  Tabs: <T extends string>(props: TabsProps<T>) => ReactNode;
};

export default DayNav;
