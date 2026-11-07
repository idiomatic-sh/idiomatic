import assert from "node:assert/strict";
import test from "node:test";
import { CompleteTodoUseCase } from "../../../../src/inbound/use_cases/complete_todo/use_case.js";
import { MemoryTodoStore } from "../../../../src/outbound/todos/adapter.js";
test("complete_todo uses its store port", async () => { const store = new MemoryTodoStore(); const useCase = new CompleteTodoUseCase(store); const added = await store.add("Milk"); assert.deepEqual(await useCase.run(added.id), { ...added, done: true }); });
