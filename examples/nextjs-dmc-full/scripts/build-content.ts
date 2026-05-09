import { build } from "@gentleduck/md";
import config from "../duck-md.config";

async function main() {
	const report = (await build(config)) as
		| { collections?: Array<{ name: string; records: number; outputPath: string }> }
		| undefined;
	const collections = report?.collections ?? [];
	if (!collections.length) {
		console.log("build complete");
		return;
	}
	for (const c of collections) {
		console.log(`${c.name}: ${c.records} records -> ${c.outputPath}`);
	}
}
main().catch((e) => {
	console.error(e);
	process.exit(1);
});
