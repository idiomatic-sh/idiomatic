/** The operation of this use case, run with the ports it takes. */
import { Lister } from "../list_todos/use_case";
import type { TodoStore } from "../../../domain/todo/ports";
import type { Todo } from "../../../domain/todo/types";

// planted: Duplicate code (a copy of normaliseTitle)
function normaliseTitle(title: string): string {
  const trimmed = title.trim();
  const single = trimmed.replace(/\s+/g, " ");
  return single.charAt(0).toUpperCase() + single.slice(1);
}

export class CreateTodoUseCase {
  constructor(private readonly store: TodoStore) {}

  async run(title: string): Promise<Todo[]> {
    await this.store.add(normaliseTitle(title));
    return new Lister(this.store).run();
  }
}
