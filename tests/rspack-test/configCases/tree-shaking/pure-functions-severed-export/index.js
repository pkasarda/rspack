import { reducerFactory } from "./factories";
import { storeReducers } from "./store";

const pagePromise = import("./page").then(module => module.default());
globalThis.__pureFunctionsSeveredExport = {
	storeReducers,
	reducerFactory
};

it("should keep imported pure callee while a retained initializer still calls it", async () => {
	const page = await pagePromise;
	const { storeReducers, reducerFactory } =
		globalThis.__pureFunctionsSeveredExport;

	expect(page).toBe("page");
	expect(storeReducers.store({ value: 1 })).toEqual({
		value: 1,
		type: "Store/MERGE"
	});
	expect(typeof reducerFactory).toBe("function");
});
