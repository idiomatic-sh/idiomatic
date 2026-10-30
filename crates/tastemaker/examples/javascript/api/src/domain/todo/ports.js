/** The store port: all(), add(title), complete(id), and archive(id), all asynchronous. */
export function requireTodoStore(store) {
  for (const method of ["all", "add", "complete", "archive"]) {
    if (typeof store[method] !== "function") {
      throw new TypeError(`todo store needs ${method}()`);
    }
  }
  return store;
}
