import { expect, test } from "vitest";
import { CompleteTodoUseCase } from "../../../../src/inbound/use_cases/complete_todo/use_case";
import { MemoryTodoStore } from "../../../../src/outbound/todos/adapter";

test("a kept todo is completed", async () => {
  const store = new MemoryTodoStore();
  const added = await store.add("Buy milk");
  expect(await new CompleteTodoUseCase(store).run(added.id)).toMatchObject({ id: added.id, done: true });
});

test("a todo that is not kept is not completed", async () => {
  const outcome = await new CompleteTodoUseCase(new MemoryTodoStore()).run(42).catch(() => undefined);
  expect(outcome).toBeUndefined();
});
