import { expect, test } from "vitest";
import { describe as describeTodo, normaliseTitle } from "../../../src/domain/todo/titles";

test("a title is trimmed, spaced once and capitalised", () => {
  expect(normaliseTitle("  buy   milk ")).toBe("Buy milk");
});

test("a description names the title and its state", () => {
  expect(describeTodo("Buy milk", true, "ann", "monday", "home")).toMatch(/^Buy milk done ann monday/);
});
