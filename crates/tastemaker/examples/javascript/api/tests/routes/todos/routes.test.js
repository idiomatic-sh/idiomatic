import assert from "node:assert/strict";
import test from "node:test";
import { makeApp } from "../../../src/main.js";

test("create, list, count, complete and archive through HTTP", async () => {
  const app = makeApp();
  const create = await app.request("/todos", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ title: "  buy   milk " }) });
  assert.equal(create.status, 201);
  const [todo] = await create.json();
  assert.deepEqual(todo, { id: 1, title: "Buy milk", done: false });
  assert.deepEqual(await (await app.request("/todos")).json(), [todo]);
  assert.deepEqual(await (await app.request("/todos/count")).json(), { count: 1 });
  const done = await app.request(`/todos/${todo.id}/done`, { method: "POST" });
  assert.equal(done.status, 200);
  assert.deepEqual(await done.json(), { ...todo, done: true });
  assert.equal((await app.request("/todos/999/done", { method: "POST" })).status, 404);
  assert.equal((await app.request("/todos/no/done", { method: "POST" })).status, 400);
  assert.equal((await app.request(`/todos/${todo.id}`, { method: "DELETE" })).status, 204);
  assert.equal((await app.request(`/todos/${todo.id}`, { method: "DELETE" })).status, 404);
  assert.deepEqual(await (await app.request("/todos/count")).json(), { count: 0 });
  assert.deepEqual(await (await makeApp().request("/todos")).json(), []);
});

test("invalid input leaves the store empty", async () => {
  const app = makeApp();
  for (const body of ["{", "null", '{"title":1}', '{"title":"   "}']) {
    assert.equal((await app.request("/todos", { method: "POST", body })).status, 400);
  }
  assert.deepEqual(await (await app.request("/todos")).json(), []);
});
