import type { ReactNode, ButtonHTMLAttributes, InputHTMLAttributes } from 'react';

export type IconName =
  | 'plus' | 'close' | 'refresh' | 'edit' | 'hide' | 'show' | 'external'
  | 'chevron-left' | 'chevron-right' | 'wifi-off' | 'alert' | 'check-circle' | 'repo';

export interface IconProps {
  /** Icon name from the set. */
  name: IconName;
  /** Pixel size, default 14. */
  size?: number;
  /** Accessible name; without it the icon is decorative (aria-hidden). */
  label?: string;
  strokeWidth?: number;
  className?: string;
}

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** primary = one per view (accent fill); secondary = default; quiet = in-row tools; dashed = "add missing". */
  variant?: 'primary' | 'secondary' | 'quiet' | 'dashed';
  /** sm 36px, md 40px, lg 44px. */
  size?: 'sm' | 'md' | 'lg';
  icon?: IconName;
  /** Icon-only square button — pass aria-label. */
  iconOnly?: boolean;
  /** Toggle state → aria-pressed. */
  pressed?: boolean;
  children?: ReactNode;
}

export interface NavItem { key: string; label: string; href?: string; active?: boolean; onClick?: () => void }
export interface AppHeaderProps {
  items: NavItem[];
  /** Formatted timer, "05:12:34". */
  clock: string;
  /** true = counting (● REC + STOP), false = stopped (START). */
  running?: boolean;
  onToggle?: () => void;
  brand?: string;
}

export interface SourceChipProps {
  /** gl GitLab, gh GitHub, my own task, mr merge request, iss issue. */
  source: 'gl' | 'gh' | 'my' | 'mr' | 'iss';
  children?: ReactNode;
  title?: string;
}

export interface FlagProps {
  /** change = amber (ZMIENIONE, Z PIĄTKU, DZIŚ); new = blue (NOWE). */
  tone?: 'change' | 'new';
  children: ReactNode;
}

export interface StatusPillProps {
  /** ok ●, calm ○ (no VPN), error ▲. */
  state: 'ok' | 'calm' | 'error';
  /** Defaults to OK / NIEDOSTĘPNY / BŁĄD. */
  children?: ReactNode;
  glyph?: boolean;
}

export interface Stat { label: string; value: string; sub?: string; tone?: 'info' | 'warn' | 'muted' }
export interface StatStripProps { items: Stat[] }

export interface PanelProps {
  title?: ReactNode;
  /** Right side of the head, mono: "[4/5]", "15 pozycji". */
  meta?: ReactNode;
  footer?: ReactNode;
  tone?: 'default' | 'danger';
  headingLevel?: 1 | 2;
  children?: ReactNode;
}

export interface ItemRowProps {
  id?: string;
  title: ReactNode;
  source?: SourceChipProps['source'];
  /** Issue / MR number: "!418", "#27". */
  refId?: string;
  flag?: ReactNode;
  flagTone?: FlagProps['tone'];
  /** Right-aligned mono meta: due date, "2 godz.". */
  meta?: ReactNode;
  done?: boolean;
  onToggle?: () => void;
  selected?: boolean;
  /** Row actions: quiet icon Buttons. */
  children?: ReactNode;
}

export interface Option { value: string; label: string }
export interface SegmentedProps { options: Option[]; value: string; onChange?: (v: string) => void; label: string }

export interface Tab { value: string; label: string; count?: number | string }
export interface TabsProps { tabs: Tab[]; value: string; onChange?: (v: string) => void; label: string }

export interface SourceStateProps {
  /** empty = data with nothing in it; calm = GitLab without VPN; error = real failure (role=alert). */
  kind: 'empty' | 'calm' | 'error';
  title: string;
  children?: ReactNode;
  /** Time of the last successful refresh. */
  lastOk?: string;
  action?: string;
  onAction?: () => void;
}

export interface FieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  /** Renders a select with these options. */
  options?: string[];
  /** Takes the free width in a wrapping form row. */
  grow?: boolean;
}

export interface HoursBarProps {
  hours: number;
  /** Norm line, default 8; 0 hides it (day off). */
  norm?: number;
  /** Bar scale, default 10 h. */
  max?: number;
  today?: boolean;
  label?: string;
}

export declare const DayNav: {
  AppHeader: (p: AppHeaderProps) => JSX.Element;
  Button: (p: ButtonProps) => JSX.Element;
  Icon: (p: IconProps) => JSX.Element;
  SourceChip: (p: SourceChipProps) => JSX.Element;
  Flag: (p: FlagProps) => JSX.Element;
  StatusPill: (p: StatusPillProps) => JSX.Element;
  StatStrip: (p: StatStripProps) => JSX.Element;
  Panel: (p: PanelProps) => JSX.Element;
  ItemRow: (p: ItemRowProps) => JSX.Element;
  Segmented: (p: SegmentedProps) => JSX.Element;
  Tabs: (p: TabsProps) => JSX.Element;
  SourceState: (p: SourceStateProps) => JSX.Element;
  Field: (p: FieldProps) => JSX.Element;
  HoursBar: (p: HoursBarProps) => JSX.Element;
};
