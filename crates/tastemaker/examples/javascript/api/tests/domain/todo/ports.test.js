import assert from "node:assert/strict";
import test from "node:test";
import { requireTodoStore } from "../../../src/domain/todo/ports.js";
import { MemoryTodoStore } from "../../../src/outbound/todos/adapter.js";
test("the port accepts its adapter and rejects missing operations", () => { const store = new MemoryTodoStore(); assert.equal(requireTodoStore(store), store); assert.throws(() => requireTodoStore({}), /needs all/); });
