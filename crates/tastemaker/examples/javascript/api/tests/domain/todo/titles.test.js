import assert from "node:assert/strict";
import test from "node:test";
import { normaliseTitle, describe } from "../../../src/domain/todo/titles.js";
test("titles are normalised", () => { assert.equal(normaliseTitle("  buy   milk "), "Buy milk"); });
test("descriptions name the todo and state", () => { assert.match(describe("Milk", true, "ann", "monday", "home"), /^Milk done ann monday/); });
