/** The operation of this use case, run with the ports it takes. */
import { HTTPException } from "hono/http-exception";
import type { TodoStore } from "../../../domain/todo/ports";
import type { Todo } from "../../../domain/todo/types";

// planted: Import direction (the use case imports the framework)
export class CompleteTodoUseCase {
  constructor(private readonly store: TodoStore) {}

  async run(id: number): Promise<Todo> {
    const done = await this.store.complete(id);
    if (done === undefined) {
      throw new HTTPException(404, { message: "no such todo" });
    }
    return done;
  }
}
