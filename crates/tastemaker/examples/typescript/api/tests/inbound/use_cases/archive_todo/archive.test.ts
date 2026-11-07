import { expect, test } from "vitest";
import { ArchiveTodoUseCase } from "../../../../src/inbound/use_cases/archive_todo/archive";

test("archiving answers the id archived", async () => {
  expect(await new ArchiveTodoUseCase().run(3)).toBe(3);
});
