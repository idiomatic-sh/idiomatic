import assert from "node:assert/strict";
import test from "node:test";
import { Lister } from "../../../../src/inbound/use_cases/list_todos/use_case.js";
import { MemoryTodoStore } from "../../../../src/outbound/todos/adapter.js";
test("list_todos uses its store port", async () => { const store = new MemoryTodoStore(); const useCase = new Lister(store); const added = await store.add("Milk"); assert.deepEqual(await useCase.run(), [added]); });
