// Generic declarations and values exercise the whole file path.
export interface Store<T> {
  get(key: string): T | undefined;
}
export function lookup<T>(store: Store<T>, key: string): T | undefined {
  return store.get(key);
}
