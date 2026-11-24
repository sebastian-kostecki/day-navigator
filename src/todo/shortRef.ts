/** `!418` / `#27` out of `group/project!418` or `owner/name#27`. */
export function shortRef(reference: string): string {
  const at = Math.max(reference.lastIndexOf("#"), reference.lastIndexOf("!"));
  return at < 0 ? reference : reference.slice(at);
}
