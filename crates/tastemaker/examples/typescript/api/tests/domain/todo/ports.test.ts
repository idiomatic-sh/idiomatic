import { expect, test } from "vitest";
import type { TodoStore } from "../../../src/domain/todo/ports";
import { MemoryTodoStore } from "../../../src/outbound/todos/adapter";

test("a store gives back what it was given", async () => {
  const store: TodoStore = new MemoryTodoStore();
  const added = await store.add("Buy milk");
  expect(await store.all()).toEqual([added]);
});
