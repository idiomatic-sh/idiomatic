import { expect, test } from "vitest";
import { routes } from "../../../src/routes/todos/routes";

test("a created todo is listed and can be completed", async () => {
  const created = await routes.request("/", { method: "POST", body: JSON.stringify({ title: "buy milk" }), headers: { "content-type": "application/json" } });
  expect(created.status).toBe(201);
  const [todo]: { id: number; title: string }[] = await created.json();
  expect(todo.title).toBe("Buy milk");
  const done = await routes.request(`/${todo.id}/done`, { method: "POST" });
  expect(done.status).toBe(200);
});

test("the count answers a number", async () => {
  const counted = await routes.request("/count");
  expect(await counted.json()).toEqual({ count: 0 });
});
