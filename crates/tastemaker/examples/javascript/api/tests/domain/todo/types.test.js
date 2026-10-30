import assert from "node:assert/strict";
import test from "node:test";
import { Todo } from "../../../src/domain/todo/types.js";
test("a todo starts open", () => { assert.deepEqual({ ...new Todo(1, "Milk") }, { id: 1, title: "Milk", done: false }); });
