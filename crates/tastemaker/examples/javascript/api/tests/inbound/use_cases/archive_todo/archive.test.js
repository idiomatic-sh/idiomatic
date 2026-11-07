import assert from "node:assert/strict";
import test from "node:test";
import { ArchiveTodoUseCase } from "../../../../src/inbound/use_cases/archive_todo/archive.js";
import { MemoryTodoStore } from "../../../../src/outbound/todos/adapter.js";
test("archive_todo uses its store port", async () => { const store = new MemoryTodoStore(); const useCase = new ArchiveTodoUseCase(store); const added = await store.add("Milk"); assert.equal(await useCase.run(added.id), true); assert.deepEqual(await store.all(), []); });
