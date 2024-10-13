// @ts-check

import test from "node:test";
import assert from "node:assert";

import { transform } from "../index.js";

test("transform works", (t) => {
	const source = "const Comp = () => <div>Hello, world!</div>;";
	assert.strictEqual(transform(source).trim(), source);
});
