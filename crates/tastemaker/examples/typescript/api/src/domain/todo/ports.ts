import type { Todo } from "./types";

/** Where todos are kept. */
export interface TodoStore {
  all(): Promise<Todo[]>;
  add(title: string): Promise<Todo>;
  complete(id: number): Promise<Todo | undefined>;
}
