import assert from "node:assert/strict";
import test from "node:test";
import { CreateTodoUseCase } from "../../../../src/inbound/use_cases/create_todo/use_case.js";
import { MemoryTodoStore } from "../../../../src/outbound/todos/adapter.js";
test("create_todo uses its store port", async () => { const store = new MemoryTodoStore(); const useCase = new CreateTodoUseCase(store); assert.deepEqual(await useCase.run("  buy   milk "), [{ id: 1, title: "Buy milk", done: false }]); });
