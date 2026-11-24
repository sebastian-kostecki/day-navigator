import type { TodoEntry, TodoItem, TodoList } from "./types";

// Item keys carry the source, so a GitHub `owner/repo#N` can never collide
// with a GitLab reference.
export function gitlabKey(kind: string, reference: string): string {
  return `gitlab:${kind}:${reference}`;
}

/** Rust stores GitHub references lowercase, so the key does too. */
export function githubKey(reference: string): string {
  return `github:${reference.toLowerCase()}`;
}

export function entryKey(entry: TodoEntry): string {
  switch (entry.kind) {
    case "own":
      return `own:${entry.taskId}`;
    case "gitlab":
      return gitlabKey(entry.itemKind, entry.reference);
    case "github":
      return githubKey(entry.reference);
  }
}

/** Key to list item, so a source view knows both "on the list" and the id to take off. */
export function itemsByKey(list: TodoList): Map<string, TodoItem> {
  const map = new Map<string, TodoItem>();
  for (const item of list.items) {
    map.set(entryKey(item.entry), item);
  }
  return map;
}
