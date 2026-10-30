/** The operation of this use case, run with the ports it takes. */
import { CreateTodoUseCase } from "../create_todo/use_case";
import type { TodoStore } from "../../../domain/todo/ports";
import type { Todo } from "../../../domain/todo/types";

// planted: Naming convention, and the import back to create_todo closes the cycle (planted: Dependency graph)
export class Lister {
  constructor(private readonly store: TodoStore) {}

  async run(): Promise<Todo[]> {
    return this.store.all();
  }

  creator(): CreateTodoUseCase {
    return new CreateTodoUseCase(this.store);
  }
}

