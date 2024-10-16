// @ts-check

import test from "node:test";
import assert from "node:assert";

import { transform } from "../index.js";

test("transform() returns compiled code", (t) => {
	assert.strictEqual(
		typeof transform("const Comp = () => <div>Hello, world!</div>;"),
		"string",
	);
});
