/** The operation of this use case, run with the ports it takes. */
// planted: Import direction
import { HTTPException } from "hono/http-exception";

export class CompleteTodoUseCase {
  constructor(store) { this.store = store; }

  async run(id) {
    const done = await this.store.complete(id);
    if (done === undefined) throw new HTTPException(404, { message: "no such todo" });
    return done;
  }
}
