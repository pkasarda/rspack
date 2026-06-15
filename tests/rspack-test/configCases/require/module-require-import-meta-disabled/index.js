import { createRequire } from "module";
import module from "module";
import * as moduleNs from "module";

it("should parse createRequire import.meta.url when importMeta parser is disabled", () => {
	const require = createRequire(import.meta.url);
	expect(require("./a")).toBe(1);
	expect(createRequire(import.meta.url)("./a")).toBe(1);
	expect(moduleNs.createRequire(import.meta.url)("./a")).toBe(1);
	expect(module.createRequire(import.meta.url)("./a")).toBe(1);
});
