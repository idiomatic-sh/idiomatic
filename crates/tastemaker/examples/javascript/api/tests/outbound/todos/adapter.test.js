import assert from "node:assert/strict";
import test from "node:test";
import { MemoryTodoStore } from "../../../src/outbound/todos/adapter.js";

test("the store owns copies and never reuses an archived id", async () => {
  const store = new MemoryTodoStore();
  const first = await store.add("Milk");
  first.title = "Changed";
  assert.equal((await store.all())[0].title, "Milk");
  await store.archive(first.id);
  assert.equal((await store.add("Eggs")).id, 2);
  assert.equal(await store.complete(99), undefined);
});
