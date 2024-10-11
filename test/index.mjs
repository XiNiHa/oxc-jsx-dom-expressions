// @ts-check

import test from "node:test";
import assert from "node:assert";

import { roundtrip } from "../index.js";

test("tsx roundtrip", (t) => {
	const source = "const Comp = () => <div>Hello, world!</div>;";
	assert.strictEqual(roundtrip(source).trim(), source);
});
