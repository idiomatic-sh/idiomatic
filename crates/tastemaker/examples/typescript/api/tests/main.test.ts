import { expect, test } from "vitest";
import app, { banner } from "../src/main";

test("the app serves the todos under /todos", async () => {
  const answer = await app.request("/todos");
  expect(answer.status).toBe(200);
  expect(await answer.json()).toEqual([]);
});

test("the banner names the api", () => {
  expect(banner("on port 3000")).toBe("todo api on port 3000");
});
