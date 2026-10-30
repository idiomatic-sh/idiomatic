export function normaliseTitle(title: string): string {
  const trimmed = title.trim();
  const single = trimmed.replace(/\s+/g, " ");
  return single.charAt(0).toUpperCase() + single.slice(1);
}

// planted: Complexity limits
export function describe(title: string, done: boolean, owner: string, due: string, tag: string): string {
  return [title, done ? "done" : "open", owner, due, tag].join(" ");
}
