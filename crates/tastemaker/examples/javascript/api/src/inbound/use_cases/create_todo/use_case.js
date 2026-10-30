/** The operation of this use case, run with the ports it takes. */
import { Lister } from "../list_todos/use_case.js";

// planted: Duplicate code
function normaliseTitle(title) {
  const trimmed = title.trim();
  const single = trimmed.replace(/\s+/g, " ");
  return single.charAt(0).toUpperCase() + single.slice(1);
}

export class CreateTodoUseCase {
  constructor(store) { this.store = store; }

  async run(title) {
    await this.store.add(normaliseTitle(title));
    return new Lister(this.store).run();
  }
}
