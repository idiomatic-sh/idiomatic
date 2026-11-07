import { expect, test } from "vitest";
import { CreateTodoUseCase } from "../../../../src/inbound/use_cases/create_todo/use_case";
import { MemoryTodoStore } from "../../../../src/outbound/todos/adapter";

test("a created todo is listed with its title normalised", async () => {
  const listed = await new CreateTodoUseCase(new MemoryTodoStore()).run("  buy   milk ");
  expect(listed.map((todo) => todo.title)).toEqual(["Buy milk"]);
});
