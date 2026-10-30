import { Hono } from "hono";
import { MemoryTodoStore } from "../../outbound/todos/adapter";
import { CreateTodoUseCase } from "../../inbound/use_cases/create_todo/use_case";
import { Lister } from "../../inbound/use_cases/list_todos/use_case";
import { CompleteTodoUseCase } from "../../inbound/use_cases/complete_todo/use_case";

const store = new MemoryTodoStore();

export const routes = new Hono();

routes.get("/", async (c) => c.json(await new Lister(store).run()));

routes.post("/", async (c) => {
  const body: { title: string } = await c.req.json();
  return c.json(await new CreateTodoUseCase(store).run(body.title), 201);
});

routes.post("/:id/done", async (c) => c.json(await new CompleteTodoUseCase(store).run(Number(c.req.param("id")))));

// planted: Framework conventions (a handler that is not async)
routes.get("/count", (c) => c.json({ count: 0 }));
