import { build } from "@gentleduck/md";
import config from "../dmc.config.js";

const report = await build(config);
for (const c of report.collections) {
	console.log(`ok ${c.name} - ${c.records} records -> ${c.outputPath}`);
}
