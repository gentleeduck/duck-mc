import { createRequire } from 'node:module';
import { readFileSync, writeFileSync, unlinkSync } from 'node:fs';
const require = createRequire(import.meta.url);
const native = require('./index.js');
const cbRegistry = new Map();
let cbId = 0;
const registerCallback = (fn) => {
    const id = ++cbId;
    cbRegistry.set(id, fn);
    return id;
};
export class SchemaBuilder {
    constructor(descriptor) {
        Object.assign(this, descriptor);
    }
    toJSON() {
        const out = { kind: this.kind };
        for (const k of Object.keys(this))
            out[k] = this[k];
        return out;
    }
    optional() { return new SchemaBuilder({ kind: 'optional', inner: this.toJSON() }); }
    nullable() { return new SchemaBuilder({ kind: 'nullable', inner: this.toJSON() }); }
    default(value) { return new SchemaBuilder({ kind: 'default', inner: this.toJSON(), fallback: value }); }
    min(n) { return new SchemaBuilder({ ...this.toJSON(), min: n }); }
    max(n) { return new SchemaBuilder({ ...this.toJSON(), max: n }); }
    length(n) { return new SchemaBuilder({ ...this.toJSON(), length: n }); }
    regex(p) { return new SchemaBuilder({ ...this.toJSON(), regex: p }); }
    int() { return new SchemaBuilder({ ...this.toJSON(), int: true }); }
    by(bucket) { return new SchemaBuilder({ ...this.toJSON(), bucket }); }
    reserved(list) { return new SchemaBuilder({ ...this.toJSON(), reserved: list }); }
    passthrough() { return new SchemaBuilder({ ...this.toJSON(), passthrough: true }); }
    transform(fn) {
        return new SchemaBuilder({ kind: 'transform', inner: this.toJSON(), __callbackId: registerCallback(fn) });
    }
    refine(fn, message) {
        return new SchemaBuilder({ kind: 'refine', inner: this.toJSON(), __callbackId: registerCallback(fn), __message: message });
    }
}
const sb = (d) => new SchemaBuilder(d);
export const s = {
    string: () => sb({ kind: 'string' }),
    number: () => sb({ kind: 'number' }),
    boolean: () => sb({ kind: 'boolean' }),
    array: (item) => sb({ kind: 'array', item: item.toJSON() }),
    object: (fields) => sb({
        kind: 'object',
        fields: Object.fromEntries(Object.entries(fields).map(([k, v]) => [k, v.toJSON()])),
    }),
    record: (value) => sb({ kind: 'record', value: value.toJSON() }),
    tuple: (items) => sb({ kind: 'tuple', items: items.map((v) => v.toJSON()) }),
    intersection: (a, b) => sb({ kind: 'intersection', left: a.toJSON(), right: b.toJSON() }),
    enum: (variants) => sb({ kind: 'enum', variants }),
    literal: (expected) => sb({ kind: 'literal', expected }),
    union: (variants) => sb({ kind: 'union', variants: variants.map((v) => v.toJSON()) }),
    discriminatedUnion: (discriminator, variants) => sb({
        kind: 'discriminatedUnion',
        discriminator,
        variants: variants.map((v) => v.toJSON()),
    }),
    coerce: {
        string: () => sb({ kind: 'coerce.string' }),
        number: () => sb({ kind: 'coerce.number' }),
        boolean: () => sb({ kind: 'coerce.boolean' }),
        date: () => sb({ kind: 'coerce.date' }),
    },
    raw: () => sb({ kind: 'raw' }),
    markdown: () => sb({ kind: 'markdown' }),
    mdx: () => sb({ kind: 'mdx' }),
    toc: () => sb({ kind: 'toc' }),
    metadata: () => sb({ kind: 'metadata' }),
    excerpt: (opts = {}) => sb({ kind: 'excerpt', ...opts }),
    path: (opts = {}) => sb({ kind: 'path', ...opts }),
    slug: (bucket, reserved) => sb({ kind: 'slug', bucket, reserved }),
    unique: (bucket) => sb({ kind: 'unique', bucket }),
    isodate: () => sb({ kind: 'isodate' }),
    file: (opts = {}) => sb({ kind: 'file', ...opts }),
    image: (opts = {}) => sb({ kind: 'image', ...opts }),
};
export const defineConfig = (config) => config;
export const defineCollection = (c) => c;
export const defineLoader = (l) => l;
export const defineSchema = (sch) => sch;
function collectCallbacks(descriptor, base = []) {
    if (!descriptor || typeof descriptor !== 'object')
        return [];
    const found = [];
    if (descriptor.kind === 'transform' && typeof descriptor.__callbackId === 'number') {
        const fn = cbRegistry.get(descriptor.__callbackId);
        if (fn)
            found.push({ path: [...base], kind: 'transform', fn });
    }
    if (descriptor.kind === 'refine' && typeof descriptor.__callbackId === 'number') {
        const fn = cbRegistry.get(descriptor.__callbackId);
        if (fn)
            found.push({ path: [...base], kind: 'refine', fn, message: descriptor.__message });
    }
    if (descriptor.inner)
        found.push(...collectCallbacks(descriptor.inner, base));
    if (descriptor.kind === 'object' && descriptor.fields) {
        for (const [k, v] of Object.entries(descriptor.fields)) {
            found.push(...collectCallbacks(v, [...base, k]));
        }
    }
    if (descriptor.kind === 'array' && descriptor.item) {
        found.push(...collectCallbacks(descriptor.item, [...base, '*']));
    }
    return found;
}
function walkPath(obj, path) {
    if (path.length === 0)
        return [];
    if (path[0] === '*') {
        if (!Array.isArray(obj))
            return [];
        return obj.flatMap((_, i) => walkPath(obj[i], path.slice(1)));
    }
    const [key, ...rest] = path;
    if (obj == null || typeof obj !== 'object' || !(key in obj))
        return [];
    if (rest.length === 0)
        return [{ parent: obj, key }];
    return walkPath(obj[key], rest);
}
function applyCallbacks(record, cbs, errors, file) {
    for (const cb of cbs) {
        for (const { parent, key } of walkPath(record, cb.path)) {
            const v = parent[key];
            if (cb.kind === 'transform') {
                try {
                    parent[key] = cb.fn(v);
                }
                catch (e) {
                    errors.push({ file, message: `${cb.path.join('.')}: transform threw: ${e.message ?? e}` });
                }
            }
            else {
                let ok = false;
                try {
                    ok = !!cb.fn(v);
                }
                catch (e) {
                    errors.push({ file, message: `${cb.path.join('.')}: refine threw: ${e.message ?? e}` });
                    continue;
                }
                if (!ok)
                    errors.push({ file, message: `${cb.path.join('.')}: ${cb.message ?? 'failed refinement'}` });
            }
        }
    }
}
function adaptToBuildInput(input) {
    if ('outputDir' in input && Array.isArray(input.collections))
        return input;
    const cfg = input;
    const root = cfg.root ?? '.';
    const outputDir = cfg.output?.data ?? '.gentleduck';
    const collections = Object.entries(cfg.collections ?? {}).map(([key, c]) => ({
        name: c.name ?? key,
        pattern: Array.isArray(c.pattern) ? c.pattern[0] : c.pattern,
        baseDir: c.baseDir ?? root,
        schema: c.schema instanceof SchemaBuilder ? c.schema.toJSON() : c.schema,
        single: c.single,
    }));
    return {
        outputDir,
        collections,
        root,
        strict: cfg.strict,
        clean: cfg.output?.clean,
        outputAssets: cfg.output?.assets,
        outputBase: cfg.output?.base,
        outputName: cfg.output?.name,
        outputFormat: cfg.output?.format,
        markdownRemarkPlugins: cfg.markdown?.remarkPlugins,
        markdownRehypePlugins: cfg.markdown?.rehypePlugins,
        mdxRemarkPlugins: cfg.mdx?.remarkPlugins,
        mdxRehypePlugins: cfg.mdx?.rehypePlugins,
        copyLinkedFiles: cfg.markdown?.copyLinkedFiles ?? cfg.mdx?.copyLinkedFiles,
        mdxOutputFormat: cfg.mdx?.outputFormat,
        mdxMinify: cfg.mdx?.minify,
        markdownGfm: cfg.markdown?.gfm,
    };
}
export function compile(source) {
    return native.compile(source);
}
export function compileMany(sources) {
    return native.compileMany(sources);
}
export async function build(input) {
    const collectionCallbacks = new Map();
    if (input?.collections && !Array.isArray(input.collections)) {
        for (const [key, c] of Object.entries(input.collections)) {
            if (c.schema) {
                const desc = c.schema instanceof SchemaBuilder ? c.schema.toJSON() : c.schema;
                const cbs = collectCallbacks(desc);
                if (cbs.length)
                    collectionCallbacks.set(c.name ?? key, cbs);
            }
        }
    }
    const report = native.build(adaptToBuildInput(input));
    const needPostprocess = collectionCallbacks.size > 0 || input.prepare || input.complete;
    if (!needPostprocess)
        return report;
    const data = {};
    for (const c of report.collections) {
        data[c.name] = JSON.parse(readFileSync(c.outputPath, 'utf8'));
    }
    for (const c of report.collections) {
        const cbs = collectionCallbacks.get(c.name);
        if (!cbs)
            continue;
        const records = Array.isArray(data[c.name]) ? data[c.name] : [data[c.name]];
        for (const record of records) {
            applyCallbacks(record, cbs, report.errors, c.outputPath);
        }
    }
    if (input.prepare) {
        const ret = await input.prepare(data, { config: input });
        if (ret === false) {
            for (const c of report.collections)
                try {
                    unlinkSync(c.outputPath);
                }
                catch { }
            return report;
        }
    }
    for (const c of report.collections) {
        writeFileSync(c.outputPath, JSON.stringify(data[c.name], null, 2));
    }
    if (input.complete)
        await input.complete(data, { config: input });
    return report;
}
export default {
    compile,
    compileMany,
    build,
    defineConfig,
    defineCollection,
    defineLoader,
    defineSchema,
    s,
    SchemaBuilder,
};
