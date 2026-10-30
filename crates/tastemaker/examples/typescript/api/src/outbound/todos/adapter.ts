import type { TodoStore } from "../../domain/todo/ports";
import type { Todo } from "../../domain/todo/types";

/** Keeps the todos in memory. */
export class MemoryTodoStore implements TodoStore {
  private readonly todos: Todo[] = [];

  async all(): Promise<Todo[]> {
    return [...this.todos];
  }

  async add(title: string): Promise<Todo> {
    const todo = { id: this.todos.length + 1, title, done: false };
    this.todos.push(todo);
    return todo;
  }

  async complete(id: number): Promise<Todo | undefined> {
    const found = this.todos.find((todo) => todo.id === id);
    if (found !== undefined) {
      found.done = true;
    }
    return found;
  }
}
