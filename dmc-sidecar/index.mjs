#!/usr/bin/env node
// User-facing walkthrough: ../dmc-docs/dmc-sidecar/
// Node helper that runs foreign remark/rehype plugins on dmc
// output. Protocol + plugin recipes live in the docs folder.

import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import rehypeRaw from "rehype-raw";
import rehypeStringify from "rehype-stringify";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import { resolve } from "node:path";
import readline from "node:readline";

const userRequire = createRequire(resolve(process.cwd(), "package.json"));

async function importFromUser(name) {
	let resolved;
	try {
		resolved = userRequire.resolve(name);
	} catch {
		return await import(name);
	}
	return await import(pathToFileURL(resolved).href);
}

async function loadPlugins(specs) {
	const out = [];
	for (const spec of specs) {
		if (typeof spec === "string") {
			const mod = await importFromUser(spec);
			out.push([mod.default ?? mod, undefined]);
		} else if (Array.isArray(spec)) {
			const [name, opts] = spec;
			const mod = await importFromUser(name);
			out.push([mod.default ?? mod, opts]);
		}
	}
	return out;
}

let cached = { key: "", proc: null };
async function buildProcessor(remarkSpecs, rehypeSpecs) {
	const key = JSON.stringify([remarkSpecs, rehypeSpecs]);
	if (cached.key === key && cached.proc) return cached.proc;
	const remarkPlugins = await loadPlugins(remarkSpecs);
	const rehypePlugins = await loadPlugins(rehypeSpecs);
	let proc = unified().use(remarkParse);
	for (const [p, opts] of remarkPlugins) proc = proc.use(p, opts);
	proc = proc.use(remarkRehype, { allowDangerousHtml: true }).use(rehypeRaw);
	for (const [p, opts] of rehypePlugins) proc = proc.use(p, opts);
	proc = proc.use(rehypeStringify, { allowDangerousHtml: true });
	cached = { key, proc };
	return proc;
}

const rl = readline.createInterface({
	input: process.stdin,
	crlfDelay: Infinity,
});
rl.on("line", async (line) => {
	if (!line) return;
	let id = null;
	try {
		const req = JSON.parse(line);
		id = req.id ?? null;
		const proc = await buildProcessor(
			req.remarkPlugins ?? [],
			req.rehypePlugins ?? [],
		);
		const file = await proc.process(req.markdown ?? "");
		process.stdout.write(
			JSON.stringify({
				id,
				html: String(file),
				messages: file.messages.map((m) => ({
					reason: m.reason,
					line: m.line,
					column: m.column,
				})),
			}) + "\n",
		);
	} catch (e) {
		process.stdout.write(
			JSON.stringify({ id, error: String(e.stack ?? e) }) + "\n",
		);
	}
});

rl.on("close", () => process.exit(0));
