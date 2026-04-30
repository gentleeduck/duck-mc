import { build } from "@duck/md";
import config from "../dmc.config.js";

const report = await build(config);
for (const c of report.collections) {
	console.log(`✓ ${c.name} — ${c.records} records → ${c.outputPath}`);
}
