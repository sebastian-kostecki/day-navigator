import type { GitlabItem } from "./types";

/** The view filter's dimensions: OR inside a dimension, AND between them. */
export type ViewDimension = "projects" | "labels" | "milestones";

export type ViewSelection = Record<ViewDimension, string[]>;

export const EMPTY_SELECTION: ViewSelection = { projects: [], labels: [], milestones: [] };

export type ViewOption = {
  value: string;
  count: number;
  color?: string | null;
  textColor?: string | null;
};

export type ViewOptions = Record<ViewDimension, ViewOption[]>;

export function isSelectionActive(sel: ViewSelection): boolean {
  return sel.projects.length > 0 || sel.labels.length > 0 || sel.milestones.length > 0;
}

/** `group/sub/project#12` → nr `#12`, project `project`, path `group/sub/project`. */
export function splitReference(reference: string): { nr: string; project: string; path: string } {
  const at = Math.max(reference.lastIndexOf("#"), reference.lastIndexOf("!"));
  if (at <= 0) return { nr: reference, project: "", path: "" };
  const path = reference.slice(0, at);
  return { nr: reference.slice(at), project: path.slice(path.lastIndexOf("/") + 1), path };
}

/** An empty dimension does not restrict; a non-empty one needs one of its values. */
export function matchesView(item: GitlabItem, sel: ViewSelection): boolean {
  if (sel.projects.length > 0 && !sel.projects.includes(splitReference(item.reference).path)) {
    return false;
  }
  if (sel.labels.length > 0 && !item.labels.some((l) => sel.labels.includes(l.name))) {
    return false;
  }
  if (
    sel.milestones.length > 0 &&
    (item.milestone === null || !sel.milestones.includes(item.milestone))
  ) {
    return false;
  }
  return true;
}

type Counter = Map<string, ViewOption>;

function bump(map: Counter, value: string, color?: string | null, textColor?: string | null) {
  const found = map.get(value);
  if (found) found.count += 1;
  else map.set(value, { value, count: 1, color, textColor });
}

function finish(map: Counter, selected: readonly string[]): ViewOption[] {
  for (const value of selected) {
    if (!map.has(value)) map.set(value, { value, count: 0 });
  }
  return [...map.values()].sort((a, b) => a.value.localeCompare(b.value, "pl"));
}

/** Values present in the items (unique by kind and reference), with counts; selected values stay listed. */
export function viewOptions(items: readonly GitlabItem[], sel: ViewSelection): ViewOptions {
  const seen = new Set<string>();
  const projects: Counter = new Map();
  const labels: Counter = new Map();
  const milestones: Counter = new Map();
  for (const item of items) {
    const key = `${item.kind}:${item.reference}`;
    if (seen.has(key)) continue;
    seen.add(key);
    const { path } = splitReference(item.reference);
    if (path) bump(projects, path);
    for (const l of item.labels) bump(labels, l.name, l.color, l.textColor);
    if (item.milestone !== null) bump(milestones, item.milestone);
  }
  return {
    projects: finish(projects, sel.projects),
    labels: finish(labels, sel.labels),
    milestones: finish(milestones, sel.milestones),
  };
}
