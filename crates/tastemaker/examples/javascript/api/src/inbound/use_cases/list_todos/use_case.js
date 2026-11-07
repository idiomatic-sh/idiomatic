/** The operation of this use case, run with the ports it takes. */
// planted: Dependency graph
import { CreateTodoUseCase } from "../create_todo/use_case.js";

// planted: Naming convention
export class Lister {
  constructor(store) { this.store = store; }

  async run() { return this.store.all(); }

  creator() { return new CreateTodoUseCase(this.store); }
}
