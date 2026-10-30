import assert from "node:assert/strict";
import test from "node:test";
import { banner, makeApp } from "../src/main.js";

test("the app mounts todos and uses the local label package", async () => {
  assert.deepEqual(await (await makeApp().request("/todos")).json(), []);
  assert.equal(banner(), "todo api on port 3000");
});
