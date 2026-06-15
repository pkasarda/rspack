import path from "path";
import { dirname as dirnameCjs, filename as filenameCjs } from "./cjs.js";
import { dirname as dirnameEsm, filename as filenameEsm } from "./esm.js";

it("should use custom name", () => {
	const stats = __STATS__.children[__STATS_I__];

	if (typeof globalThis.document === "undefined") {
		const filename = path.join(
			stats.outputPath,
			`./bundle${__STATS_I__}.${__STATS_I__ === 0 ? "js" : "mjs"}`
		);

		expect(dirnameCjs).toBe(stats.outputPath);
		expect(filenameCjs).toBe(filename);
		expect(dirnameEsm).toBe(stats.outputPath);
		expect(filenameEsm).toBe(filename);
	} else {
		const extension = __STATS_I__ === 2 ? "js" : "mjs";

		expect(dirnameCjs).toBe(stats.outputPath);
		expect(filenameCjs.endsWith(`bundle${__STATS_I__}.${extension}`)).toBe(
			true
		);
		expect(dirnameEsm).toBe(stats.outputPath);
		expect(filenameEsm.endsWith(`bundle${__STATS_I__}.${extension}`)).toBe(
			true
		);
	}
});
