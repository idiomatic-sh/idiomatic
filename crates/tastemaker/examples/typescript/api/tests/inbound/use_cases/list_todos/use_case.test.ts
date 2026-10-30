import { expect, test } from "vitest";
import { Lister } from "../../../../src/inbound/use_cases/list_todos/use_case";
import { MemoryTodoStore } from "../../../../src/outbound/todos/adapter";

test("every kept todo is listed", async () => {
  const store = new MemoryTodoStore();
  const added = await store.add("Buy milk");
  expect(await new Lister(store).run()).toEqual([added]);
});
