import { Hono } from "hono";
import { requireTodoStore } from "../../domain/todo/ports.js";
import { CreateTodoUseCase } from "../../inbound/use_cases/create_todo/use_case.js";
import { Lister } from "../../inbound/use_cases/list_todos/use_case.js";
import { CompleteTodoUseCase } from "../../inbound/use_cases/complete_todo/use_case.js";
import { ArchiveTodoUseCase } from "../../inbound/use_cases/archive_todo/archive.js";

export function makeRoutes(store) {
  requireTodoStore(store);
  const routes = new Hono();
  routes.get("/", async (c) => c.json(await new Lister(store).run()));
  routes.post("/", async (c) => {
    let body;
    try { body = await c.req.json(); } catch { return c.json({ message: "invalid JSON" }, 400); }
    if (typeof body?.title !== "string" || !body.title.trim()) return c.json({ message: "a todo needs a title" }, 400);
    return c.json(await new CreateTodoUseCase(store).run(body.title), 201);
  });
  routes.post("/:id/done", async (c) => {
    const id = Number(c.req.param("id"));
    if (!Number.isSafeInteger(id) || id < 1) return c.json({ message: "invalid id" }, 400);
    const done = await new CompleteTodoUseCase(store).run(id);
    return done === undefined ? c.json({ message: "no such todo" }, 404) : c.json(done);
  });
  routes.delete("/:id", async (c) => {
    const removed = await new ArchiveTodoUseCase(store).run(Number(c.req.param("id")));
    return removed ? c.body(null, 204) : c.json({ message: "no such todo" }, 404);
  });
  // planted: Framework conventions
  routes.get("/count", (c) => store.all().then(async (todos) => c.json({ count: todos.length })));
  return routes;
}
