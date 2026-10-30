import { expect, test } from "vitest";
import { MemoryTodoStore } from "../../../src/outbound/todos/adapter";

test("each todo gets its own id and completes", async () => {
  const store = new MemoryTodoStore();
  const first = await store.add("Buy milk");
  const second = await store.add("Call ann");
  expect(first.id).not.toBe(second.id);
  expect((await store.complete(second.id))?.done).toBe(true);
  expect(await store.complete(99)).toBeUndefined();
});
