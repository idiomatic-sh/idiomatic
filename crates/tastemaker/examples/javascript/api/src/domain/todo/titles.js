export function normaliseTitle(title) {
  const trimmed = title.trim();
  const single = trimmed.replace(/\s+/g, " ");
  return single.charAt(0).toUpperCase() + single.slice(1);
}

// planted: Complexity limits
export function describe(title, done, owner, due, tag) {
  return [title, done ? "done" : "open", owner, due, tag].join(" ");
}
