// One mermaid renderer for a whole build: a single headless browser draws
// every diagram, where `mmdc` launches node and Chrome once per diagram and
// theme. Spawned by `renderer.rs`; one JSON object per line each way.
//
// argv: <mermaid-cli package dir> [puppeteer config file]
// out, once: {"ready":true} | {"ready":false,"error":"..."}
// in:        {"id","source","theme","backgroundColor","config"}
// out:       {"id","svg"} | {"id","error"}, in completion order

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { pathToFileURL } from "node:url";

// Close the browser once nothing has been asked for this long, so a
// long-lived host (a dev server) does not keep Chrome around. The next
// diagram starts a fresh renderer.
const IDLE_EXIT_MS = 30_000;

const send = (msg) => process.stdout.write(`${JSON.stringify(msg)}\n`);

// The same code `mmdc` runs: its own `renderMermaid`, and its own puppeteer
// with the launch options it builds (its default, the config file over it).
async function start(cliDir, puppeteerConfigFile) {
	const cliRequire = createRequire(join(cliDir, "package.json"));
	const { renderMermaid } = await import(pathToFileURL(cliRequire.resolve("@mermaid-js/mermaid-cli")).href);
	const pm = await import(pathToFileURL(cliRequire.resolve("puppeteer")).href);
	const puppeteer = pm.default?.launch ? pm.default : pm;
	const launch = { headless: "shell" };
	if (puppeteerConfigFile) Object.assign(launch, JSON.parse(readFileSync(puppeteerConfigFile, "utf8")));
	return { renderMermaid, browser: await puppeteer.launch(launch) };
}

function serve({ renderMermaid, browser }) {
	let closing = false;
	const shutdown = async () => {
		closing = true;
		await browser.close().catch(() => {});
		process.exit(0);
	};
	let inFlight = 0;
	let idle = setTimeout(shutdown, IDLE_EXIT_MS);

	// One diagram per compile thread at a time: dmc waits on each reply, so
	// the pages open here never outnumber its threads.
	const lines = createInterface({ input: process.stdin });
	lines.on("line", async (line) => {
		// Never answered: dmc sees this process exit and retries the
		// diagram on a fresh renderer, rather than reporting it as broken.
		if (closing) return;
		clearTimeout(idle);
		inFlight++;
		let id;
		try {
			const job = JSON.parse(line);
			id = job.id;
			// Exactly what `mmdc --input - --output - --outputFormat svg` passes
			// for the flags dmc sets; everything else is the CLI's default.
			const { data } = await renderMermaid(browser, job.source, "svg", {
				mermaidConfig: { theme: job.theme, ...job.config },
				backgroundColor: job.backgroundColor,
				viewport: { width: 800, height: 600, deviceScaleFactor: 1 },
				iconPacks: [],
				iconPacksNamesAndUrls: [],
			});
			send({ id, svg: new TextDecoder().decode(data) });
		} catch (e) {
			send({ id, error: String(e?.message ?? e) });
		} finally {
			if (--inFlight === 0) idle = setTimeout(shutdown, IDLE_EXIT_MS);
		}
	});
	// stdin closes when dmc exits, killed or not: take the browser down with it.
	lines.on("close", shutdown);
	send({ ready: true });
}

const [cliDir, puppeteerConfigFile] = process.argv.slice(1);
let started;
try {
	started = await start(cliDir, puppeteerConfigFile);
} catch (e) {
	// No exit call: node exits on its own once this line is flushed.
	send({ ready: false, error: String(e?.message ?? e) });
}
if (started) serve(started);
