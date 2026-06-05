import { getCjsFoo } from "./cjs-user";
import { getNamespaceValue } from "./namespace-user";
import { foo } from "./lib";
import { getJsonName } from "./json-user";

it("should collect rsdoctor export usage graph", () => {
	expect(foo()).toBe(42);
	expect(getJsonName()).toBe("rspack");
	expect(getCjsFoo()).toBe(3);
	expect(getNamespaceValue()).toBe("ns-value");
});
