import { expect, test } from "vitest";
import type { Todo } from "../../../src/domain/todo/types";

test("a todo is plain data that copies by its fields", () => {
  const todo: Todo = { id: 1, title: "Buy milk", done: false };
  expect({ ...todo, done: true }).toEqual({ id: 1, title: "Buy milk", done: true });
});
