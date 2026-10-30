import { Todo } from "../../domain/todo/types.js";

export class MemoryTodoStore {
  #todos = [];
  #nextID = 1;

  async all() { return this.#todos.map((todo) => ({ ...todo })); }

  async add(title) {
    const todo = new Todo(this.#nextID++, title);
    this.#todos.push(todo);
    return { ...todo };
  }

  async complete(id) {
    // planted: Idiomatic code
    const todo = this.#todos.find((candidate) => candidate.id == id);
    if (todo !== undefined) todo.done = true;
    return todo === undefined ? undefined : { ...todo };
  }

  async archive(id) {
    const index = this.#todos.findIndex((todo) => todo.id === id);
    if (index < 0) return false;
    this.#todos.splice(index, 1);
    return true;
  }
}
