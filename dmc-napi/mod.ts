// User-facing walkthrough: ../dmc-docs/dmc-napi/
// TS module surface for @gentleduck/md (npm). The Rust engine
// lives in dmc-core; this file is the JS-facing API + cache layer.

import { createRequire } from "node:module";
import {
	readFileSync,
	writeFileSync,
	unlinkSync,
	readdirSync,
	statSync,
	existsSync,
} from "node:fs";
import { join, relative, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import type { Plugin, Pluggable } from "unified";

export type { Plugin, Pluggable } from "unified";

/**
 * One unified-style plugin that operates on the mdast tree before dmc
 * parses the document. Tighter than `Pluggable` — strings and nested
 * `PluggableList` are excluded since the preMdx pipeline runs each
 * entry directly, so neither makes sense here.
 *
 * The first generic carries the plugin's options tuple so the
 * `[plugin, options]` form gives TS errors on misspelled / wrong-shape
 * config. We deliberately leave the tree type as the unified default
 * (`any`) — consumer plugins routinely declare their own narrower tree
 * (e.g. `IUnistTree`) and a stricter tree generic here would reject
 * them via contravariance.
 *
 * ```ts
 * preMdxPlugins: [
 *   rehypeComponent,                                // Plugin<[]>
 *   [rehypeAutolinkHeadings, { behavior: 'wrap' }], // tuple form
 * ]
 * ```
 */
// biome-ignore lint/suspicious/noExplicitAny: variadic + tree generic
export type PreMdxPlugin<Options extends [any?, ...any[]] = [any?]> =
	// biome-ignore lint/suspicious/noExplicitAny: tree shape is consumer-defined
	| Plugin<Options, any, any>
	// biome-ignore lint/suspicious/noExplicitAny: tree shape is consumer-defined
	| [Plugin<Options, any, any>, ...Options];

/**
 * Type-safe `[plugin, options]` tuple. The `options` argument is inferred
 * from the plugin function's first parameter type, so misspelled or wrong-shape
 * options surface as TS errors at config time.
 *
 * Usage:
 *   rehypePlugins: [
 *     rehypeSlug,
 *     definePlugin(rehypePrettyCode, { theme: { light, dark } }),
 *     definePlugin(rehypeAutolinkHeadings, { properties: {...} }),
 *   ]
 */
export function definePlugin<P extends Plugin>(plugin: P): P;
export function definePlugin<
	P extends Plugin<Params, any, any>,
	// biome-ignore lint/suspicious/noExplicitAny: variadic constraint
	Params extends [any?, ...any[]] = P extends Plugin<infer X, any, any>
		? X
		: never,
>(plugin: P, options: Params[0]): [P, Params[0]];
export function definePlugin<P extends Plugin>(
	plugin: P,
	options?: unknown,
): P | [P, unknown] {
	return options === undefined ? plugin : [plugin, options];
}

const require = createRequire(import.meta.url);
const native = require("./index.js");

// Resolve sidecar entry relative to @gentleduck/md package + propagate via env
function resolveSidecar(): string | null {
	const here = dirname(fileURLToPath(import.meta.url));
	const candidates = [
		join(here, "..", "dmc-sidecar", "index.mjs"),
		join(here, "..", "..", "dmc-sidecar", "index.mjs"),
		join(here, "..", "node_modules", "@duck", "md-sidecar", "index.mjs"),
	];
	for (const p of candidates) if (existsSync(p)) return p;
	return null;
}
const SIDECAR_PATH = resolveSidecar();
if (SIDECAR_PATH && !process.env.dmc_SIDECAR) {
	process.env.dmc_SIDECAR = SIDECAR_PATH;
}

export interface TocItem {
	title: string;
	url: string;
	items: TocItem[];
}

export interface Metadata {
	readingTime: number;
	wordCount: number;
}

export interface CompileOutput {
	body: string;
	content: string;
	html: string;
	excerpt: string;
	metadata: Metadata;
	toc: TocItem[];
	frontmatter: unknown;
	frontmatterRaw: string;
	imports: string[];
	exports: string[];
}

export type SchemaKind =
	| "string"
	| "number"
	| "boolean"
	| "array"
	| "object"
	| "record"
	| "tuple"
	| "intersection"
	| "enum"
	| "literal"
	| "union"
	| "discriminatedUnion"
	| "optional"
	| "nullable"
	| "default"
	| "transform"
	| "refine"
	| "superRefine"
	| "coerce.string"
	| "coerce.number"
	| "coerce.boolean"
	| "coerce.date"
	| "raw"
	| "markdown"
	| "mdx"
	| "toc"
	| "metadata"
	| "excerpt"
	| "path"
	| "slug"
	| "unique"
	| "isodate"
	| "file"
	| "image";

export interface SchemaDescriptor {
	kind: SchemaKind;
	[field: string]: unknown;
}

export interface CollectionConfig<S = unknown> {
	name?: string;
	pattern: string | string[];
	baseDir?: string;
	single?: boolean;
	/**
	 * Schema for records in this collection. Always built via the `s`
	 * helpers (`s.object({...})`, etc) — those return Standard Schema v1
	 * compliant `SchemaBuilder<T>` instances, so downstream tooling can
	 * read `schema['~standard']['types']['output']` for record types.
	 */
	schema?: SchemaBuilder<S>;
}

export interface OutputOptions {
	data?: string;
	assets?: string;
	base?: string;
	name?: string;
	clean?: boolean;
	format?: "esm" | "cjs";
	/** Emit a per-record `html` field rendered by the native pipeline (no
	 * sidecar/JS plugins). Set to `false` (default) when you only want the
	 * MDX body / JSX tree and will render via React. */
	html?: boolean;
}

/**
 * Bundled syntect theme names. Listed for autocomplete/discovery —
 * the trailing `(string & {})` keeps the type open to any other theme
 * the syntect bundle may add without forcing a type bump.
 */
export type PrettyCodeBundledTheme =
	| "Catppuccin Latte"
	| "Catppuccin Mocha"
	| "Catppuccin Frappe"
	| "Catppuccin Macchiato"
	| "Nord"
	| "One Dark"
	| "Solarized Light"
	| "Solarized Dark"
	| "InspiredGitHub"
	| "GitHub"
	| "github-light"
	| "github-dark"
	| "base16-ocean.dark"
	| "base16-ocean.light"
	| "base16-eighties.dark"
	| "base16-mocha.dark"
	| "Tomorrow"
	| "Tomorrow Night"
	| "Monokai"
	| "Dracula"
	// Forward-compat: any other syntect-bundled theme name.
	| (string & {});

/**
 * How multi-theme pretty-code output is laid out in the DOM.
 *
 * - `"css-vars"` (default, fast, half the AST): single `<pre><code>`
 *   tree. Each token carries `style="--dmc-light:#XXX;--dmc-dark:#YYY"`.
 *   Consumer CSS swaps themes by overriding `color` to
 *   `var(--dmc-{active})`. ~25% faster than `"split"`.
 * - `"split"` (velite parity): one `<pre data-theme="<mode>">` subtree
 *   per theme, each with solid `color:#XXX` per token. Consumer CSS
 *   shows/hides whole panes via `[data-theme]`.
 */
export type MultiThemeStrategy = "css-vars" | "split";

/**
 * Per-mode pretty-code theme map. The well-known `light` / `dark` keys
 * surface in editor autocomplete; any other key is accepted (e.g.
 * `dim`, `night`) and emits its own `<pre data-theme="<key>">`.
 */
export interface PrettyCodeThemeMap {
	light?: PrettyCodeBundledTheme;
	dark?: PrettyCodeBundledTheme;
	dim?: PrettyCodeBundledTheme;
	night?: PrettyCodeBundledTheme;
	[mode: string]: PrettyCodeBundledTheme | undefined;
}

/**
 * Pretty-code theme spec. Single bundled theme name OR `{ mode: theme }`
 * map for multi-mode (light/dark) output. Default:
 * `{ light: "Catppuccin Latte", dark: "Catppuccin Mocha" }`.
 */
export type PrettyCodeTheme = PrettyCodeBundledTheme | PrettyCodeThemeMap;

/**
 * Pretty-code (syntax highlighter) configuration. Every field is
 * optional — `undefined` keeps the bundled default.
 *
 * Output shape mirrors `rehype-pretty-code`:
 * `<div data-rehype-pretty-code-fragment>` → optional `<figcaption>` →
 * one `<pre __rawString__ data-language data-theme>` per theme.
 */
export interface PrettyCodeOptions {
	/** Theme spec — single string or `{ mode: theme }` map. */
	theme?: PrettyCodeTheme;
	/**
	 * Mode whose colors fill unprefixed `color` / `background-color`.
	 * Default: `"dark"` if present, else first key.
	 */
	defaultMode?: string;
	/**
	 * Multi-theme DOM strategy. Default `"css-vars"` (single tree, faster,
	 * half the AST). Consumer CSS swaps `color` to `var(--dmc-{active})`
	 * to switch themes. Set to `"split"` for velite parity (one
	 * `<pre data-theme="…">` per theme).
	 */
	multiThemeStrategy?: MultiThemeStrategy;
	/** Keep `__rawString__` on `<pre>` for Copy button. Default `true`. */
	keepRawString?: boolean;
	/** Wrap with `<div data-rehype-pretty-code-fragment="">`. Default `true`. */
	fragmentWrapper?: boolean;
	/** Class on each line `<span>`. Default `"line"`. */
	lineClass?: string;
	/** Attribute on lines listed in `{1,3-5}` meta. Default `"data-highlighted-line"`. */
	highlightedLineAttr?: string;
	/** Language used when fence has no lang. Default `"plaintext"`. */
	defaultLanguage?: string;
	/** Unknown langs render as plaintext. Default `true`. */
	fallbackToPlaintext?: boolean;
	/** Render `<figcaption>` from `title="…"` meta. Default `true`. */
	renderTitle?: boolean;
	/** Include `data-language` on `<pre>` + `<code>`. Default `true`. */
	includeDataLanguage?: boolean;
	/**
	 * Emit a solid `background-color` on `<pre>` from the primary theme.
	 * Default `true`. Set `false` to skip the inline bg so the outer
	 * `[data-dmc-fragment]` wrapper (or consumer chrome) owns the
	 * surface color. Per-mode `--dmc-{mode}-bg` custom properties are
	 * always emitted, so CSS can still opt back in via
	 * `var(--dmc-{mode}-bg)`.
	 */
	includePreBackground?: boolean;
	/** Languages to skip (passed through unchanged). `mermaid` is always skipped. */
	skipLanguages?: string[];
	/** Expand tab characters to N spaces before highlighting. */
	tabSize?: number;
}

/**
 * CSS dimension that mermaid accepts as either a numeric pixel value
 * (e.g. `14`) or a string with units (e.g. `"14px"`, `"1.2em"`).
 */
export type CssDimension = number | string;

/**
 * CSS font-weight: numeric (`400`, `700`) or named string
 * (`"normal"`, `"bold"`).
 */
export type CssFontWeight = number | "normal" | "bold" | "lighter" | "bolder" | (string & {});

/** Bundled mermaid theme names (`mmdc --theme`). */
export type MermaidThemeName =
	| "default"
	| "dark"
	| "forest"
	| "neutral"
	| "base"
	| "null";

/**
 * Per-mode mermaid theme map. The well-known `light` / `dark` keys
 * surface in editor autocomplete; any other key is accepted (e.g.
 * `dim`, `night`) and emits its own `${key}Svg` attr on
 * `<MermaidDiagram>`.
 */
export interface MermaidThemeMap {
	light?: MermaidThemeName;
	dark?: MermaidThemeName;
	dim?: MermaidThemeName;
	night?: MermaidThemeName;
	[mode: string]: MermaidThemeName | undefined;
}

/**
 * Mermaid theme spec. Either a single theme name (one render, single
 * `chartSvg` attr emitted on `<MermaidDiagram>`) or a `mode -> theme`
 * map for per-mode rendering. Default: `{ light: "default", dark: "dark" }`
 * which keeps `lightSvg` / `darkSvg` attrs on the JSX node.
 */
export type MermaidThemeMode = MermaidThemeName | MermaidThemeMap;

/**
 * Common mermaid `themeVariables` keys. Mermaid accepts any string
 * key; the listed ones are the documented "official" variables.
 * Forward-compat: extra keys are tolerated via the index signature.
 */
export interface MermaidThemeVariables {
	background?: string;
	fontFamily?: string;
	fontSize?: string;
	primaryColor?: string;
	primaryTextColor?: string;
	primaryBorderColor?: string;
	secondaryColor?: string;
	secondaryTextColor?: string;
	secondaryBorderColor?: string;
	tertiaryColor?: string;
	tertiaryTextColor?: string;
	tertiaryBorderColor?: string;
	noteBkgColor?: string;
	noteTextColor?: string;
	noteBorderColor?: string;
	lineColor?: string;
	textColor?: string;
	mainBkg?: string;
	errorBkgColor?: string;
	errorTextColor?: string;
	nodeBkg?: string;
	nodeBorder?: string;
	clusterBkg?: string;
	clusterBorder?: string;
	defaultLinkColor?: string;
	titleColor?: string;
	edgeLabelBackground?: string;
}

/** Mermaid `flowchart` block. */
export interface MermaidFlowchartConfig {
	htmlLabels?: boolean;
	useMaxWidth?: boolean;
	defaultRenderer?: "dagre-d3" | "dagre-wrapper" | "elk";
	curve?: "basis" | "linear" | "cardinal" | "stepBefore" | "stepAfter" | "natural" | "monotoneX" | "monotoneY";
	diagramPadding?: number;
	nodeSpacing?: number;
	rankSpacing?: number;
	padding?: number;
	titleTopMargin?: number;
	subGraphTitleMargin?: { top?: number; bottom?: number };
	wrappingWidth?: number;
	arrowMarkerAbsolute?: boolean;
}

/** Mermaid `sequence` block. */
export interface MermaidSequenceConfig {
	useMaxWidth?: boolean;
	hideUnusedParticipants?: boolean;
	activationWidth?: number;
	diagramMarginX?: number;
	diagramMarginY?: number;
	actorMargin?: number;
	width?: number;
	height?: number;
	boxMargin?: number;
	boxTextMargin?: number;
	noteMargin?: number;
	messageMargin?: number;
	messageAlign?: "left" | "center" | "right";
	mirrorActors?: boolean;
	forceMenus?: boolean;
	bottomMarginAdj?: number;
	rightAngles?: boolean;
	showSequenceNumbers?: boolean;
	actorFontSize?: CssDimension;
	actorFontFamily?: string;
	actorFontWeight?: CssFontWeight;
	noteFontSize?: CssDimension;
	noteFontFamily?: string;
	noteFontWeight?: CssFontWeight;
	noteAlign?: "left" | "center" | "right";
	messageFontSize?: CssDimension;
	messageFontFamily?: string;
	messageFontWeight?: CssFontWeight;
	wrap?: boolean;
	wrapPadding?: number;
	labelBoxWidth?: number;
	labelBoxHeight?: number;
}

/** Mermaid `gantt` block. */
export interface MermaidGanttConfig {
	useMaxWidth?: boolean;
	titleTopMargin?: number;
	barHeight?: number;
	barGap?: number;
	topPadding?: number;
	rightPadding?: number;
	leftPadding?: number;
	gridLineStartPadding?: number;
	fontSize?: number;
	sectionFontSize?: CssDimension;
	numberSectionStyles?: number;
	axisFormat?: string;
	tickInterval?: string;
	topAxis?: boolean;
	displayMode?: "" | "compact";
	weekday?: "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday" | "sunday";
}

/** Mermaid `er` block. */
export interface MermaidErConfig {
	useMaxWidth?: boolean;
	titleTopMargin?: number;
	diagramPadding?: number;
	layoutDirection?: "TB" | "BT" | "LR" | "RL";
	minEntityWidth?: number;
	minEntityHeight?: number;
	entityPadding?: number;
	stroke?: string;
	fill?: string;
	fontSize?: number;
}

/** Mermaid `pie` block. */
export interface MermaidPieConfig {
	useMaxWidth?: boolean;
	textPosition?: number;
}

/** Mermaid `class` / `state` block. */
export interface MermaidNodeRendererConfig {
	useMaxWidth?: boolean;
	titleTopMargin?: number;
	defaultRenderer?: "dagre-d3" | "dagre-wrapper" | "elk";
	arrowMarkerAbsolute?: boolean;
	dividerMargin?: number;
	padding?: number;
	textHeight?: number;
}

/** Mermaid `gitGraph` block. */
export interface MermaidGitGraphConfig {
	useMaxWidth?: boolean;
	titleTopMargin?: number;
	diagramPadding?: number;
	nodeLabel?: { width?: number; height?: number; x?: number; y?: number };
	mainBranchName?: string;
	mainBranchOrder?: number;
	showCommitLabel?: boolean;
	showBranches?: boolean;
	rotateCommitLabel?: boolean;
	parallelCommits?: boolean;
}

/** Mermaid `journey` block. */
export interface MermaidJourneyConfig {
	useMaxWidth?: boolean;
	diagramMarginX?: number;
	diagramMarginY?: number;
	leftMargin?: number;
	width?: number;
	height?: number;
	boxMargin?: number;
	boxTextMargin?: number;
	noteMargin?: number;
	messageMargin?: number;
	messageAlign?: "left" | "center" | "right";
	bottomMarginAdj?: number;
	rightAngles?: boolean;
	taskFontSize?: CssDimension;
	taskFontFamily?: string;
	taskMargin?: number;
	activationWidth?: number;
	textPlacement?: string;
}

/** Mermaid `mindmap` block. */
export interface MermaidMindmapConfig {
	useMaxWidth?: boolean;
	padding?: number;
	maxNodeWidth?: number;
}

/** Mermaid `timeline` block. */
export interface MermaidTimelineConfig {
	useMaxWidth?: boolean;
	disableMulticolor?: boolean;
}

/** Mermaid `sankey` block. */
export interface MermaidSankeyConfig {
	useMaxWidth?: boolean;
	nodeAlignment?: "left" | "right" | "center" | "justify";
	showValues?: boolean;
}

/** Mermaid `xyChart` block. */
export interface MermaidXyChartConfig {
	useMaxWidth?: boolean;
	width?: number;
	height?: number;
}

/** Mermaid `block` block. */
export interface MermaidBlockConfig {
	useMaxWidth?: boolean;
	padding?: number;
}

/** Mermaid `requirement` block. */
export interface MermaidRequirementConfig {
	useMaxWidth?: boolean;
	rect_min_width?: number;
	rect_min_height?: number;
}

/** Mermaid `c4` block. */
export interface MermaidC4Config {
	useMaxWidth?: boolean;
	diagramMarginX?: number;
	diagramMarginY?: number;
}

/** Mermaid `architecture` block. */
export interface MermaidArchitectureConfig {
	useMaxWidth?: boolean;
	padding?: number;
	iconSize?: number;
}

/** Mermaid `radar` block. */
export interface MermaidRadarConfig {
	useMaxWidth?: boolean;
}

/** Mermaid `treemap` block. */
export interface MermaidTreemapConfig {
	useMaxWidth?: boolean;
	padding?: number;
}

/**
 * Full mermaid render config. Single flat object — every
 * `mermaid.initialize()` knob (themeVariables, flowchart, sequence,
 * gantt, look, layout, …) lives at the top level alongside the
 * dmc-side render knobs (responsiveSvg, centerLabels, outputDir, …).
 *
 * Every field is optional; an empty object keeps the bundled defaults
 * (light + dark themes, htmlLabels:false, flowchart spacing).
 */
export interface MermaidOptions {
	// dmc render knobs
	/**
	 * Theme spec — single string (one render → `chartSvg` attr) or
	 * `{ mode: theme }` map (per-mode render → `${mode}Svg` per entry).
	 * Default `{ light: "default", dark: "dark" }` keeps the historical
	 * `lightSvg` + `darkSvg` shape.
	 */
	theme?: MermaidThemeMode;
	/** `mmdc --backgroundColor`. Default `"transparent"`. */
	backgroundColor?: string;
	/**
	 * Apply the responsive-width post-process (rewrite first
	 * `width="..."` on the root SVG to `width="100%"`). Default `true`.
	 */
	responsiveSvg?: boolean;
	/**
	 * Inject `text-anchor="middle"` on label `<text>` / `<tspan>` so
	 * flowchart node labels center inside their `<rect>` when
	 * `htmlLabels: false` is in effect. Default `true`.
	 */
	centerLabels?: boolean;
	/** Disk SVG cache directory. */
	outputDir?: string;
	/** Forwarded to `mmdc --puppeteerConfigFile`. */
	puppeteerConfigFile?: string;

	// mermaid.initialize()
	/**
	 * Override the bundled `htmlLabels: false` default. `true` switches
	 * flowchart node labels back to HTML-in-`<foreignObject>`.
	 */
	htmlLabels?: boolean;
	themeVariables?: MermaidThemeVariables;
	fontFamily?: string;
	fontSize?: number;
	startOnLoad?: boolean;
	arrowMarkerAbsolute?: boolean;
	deterministicIds?: boolean;
	deterministicIDSeed?: string;
	maxTextSize?: number;
	maxEdges?: number;
	securityLevel?: "strict" | "loose" | "antiscript" | "sandbox";
	logLevel?: "debug" | "info" | "warn" | "error" | "fatal" | 1 | 2 | 3 | 4 | 5;
	look?: "classic" | "neo" | "handDrawn";
	layout?: "dagre" | "elk";
	handDrawnSeed?: number;
	wrap?: boolean;
	dompurifyConfig?: Record<string, unknown>;
	flowchart?: MermaidFlowchartConfig;
	sequence?: MermaidSequenceConfig;
	gantt?: MermaidGanttConfig;
	er?: MermaidErConfig;
	pie?: MermaidPieConfig;
	class?: MermaidNodeRendererConfig;
	state?: MermaidNodeRendererConfig;
	gitGraph?: MermaidGitGraphConfig;
	journey?: MermaidJourneyConfig;
	mindmap?: MermaidMindmapConfig;
	timeline?: MermaidTimelineConfig;
	sankey?: MermaidSankeyConfig;
	xyChart?: MermaidXyChartConfig;
	block?: MermaidBlockConfig;
	requirement?: MermaidRequirementConfig;
	c4?: MermaidC4Config;
	architecture?: MermaidArchitectureConfig;
	radar?: MermaidRadarConfig;
	treemap?: MermaidTreemapConfig;
}

/**
 * Sidecar-routable plugin name. dmc recognises these JS plugin names
 * for `forceSidecar` / `preferSidecar`; passing one drops the matching
 * native transformer and routes work through the sidecar.
 *
 * The `(string & {})` tail keeps the union open to plugin names dmc
 * doesn't yet recognise — the sidecar still receives them, they just
 * don't toggle a native opt-out.
 */
export type SidecarPluginName =
	| "remark-gfm"
	| "remark-math"
	| "remark-emoji"
	| "remark-mermaid"
	| "rehype-pretty-code"
	| "shiki"
	| "rehype-katex"
	| "rehype-mathjax"
	| "rehype-slug"
	| "rehype-autolink-headings"
	| "mermaid"
	| "rehype-mermaid"
	| (string & {});

/**
 * Single content-pipeline config, applied to both `.md` and `.mdx`
 * files. The MDX-only knobs (`outputFormat`, `minify`) are no-ops for
 * plain markdown.
 */
export interface ContentOptions {
	gfm?: boolean;
	copyLinkedFiles?: boolean;
	remarkPlugins?: Pluggable[];
	rehypePlugins?: Pluggable[];
	/**
	 * Source-level remark/mdast plugins that run **before** dmc parses.
	 * Each `.mdx` file is read, processed through
	 * `unified(remark-parse + remark-mdx + preMdxPlugins +
	 * remark-stringify-mdx)` and the rewritten source is fed to dmc's
	 * native pipeline. Use this slot for source-mutating plugins (e.g.
	 * `<ComponentSource>` resolvers that inject fenced code blocks as
	 * JSX children) so the inserted blocks get highlighted natively by
	 * dmc's PrettyCode transformer.
	 *
	 * Plugins receive an mdast tree with MDX nodes:
	 * `mdxJsxFlowElement` / `mdxJsxTextElement` carry `name` +
	 * `attributes`, JSX children appear as nested mdast nodes.
	 */
	preMdxPlugins?: PreMdxPlugin[];
	/**
	 * Extra paths whose content participates in the preMdx cache key.
	 * The mirror under `<output>/.cache/preprocessed/` is keyed on
	 * `(sourceHash + pluginsHash + extraInputsHash)` per file: when
	 * none of the three has changed since the last build, the cached
	 * mirror entry is reused and the plugins don't run for that file.
	 *
	 * Declare anything outside the source `.mdx` itself that your
	 * preMdx plugins read (registry index JSON, sidebars, generated
	 * tables, …). Globs are not supported — list concrete file paths
	 * (relative to the build's `cwd` or absolute). Missing files are
	 * silently skipped so a fresh checkout doesn't error.
	 *
	 * Defaults to `[]` (only source content + plugin module hashes
	 * gate the cache).
	 */
	preMdxCacheInputs?: string[];
	/** Mermaid render config. `undefined` keeps bundled defaults. */
	mermaid?: MermaidOptions;
	/** Pretty-code (syntax highlighter) config. `undefined` keeps defaults. */
	prettyCode?: PrettyCodeOptions;
	/**
	 * Bypass the plugin gate for every plugin: every JS plugin runs in
	 * the sidecar, every native transformer is dropped.
	 */
	forceSidecar?: boolean;
	/**
	 * Per-plugin sidecar preference. Names listed here run in the
	 * sidecar; the matching native transformer is dropped from the
	 * pipeline. See [`SidecarPluginName`] for the recognised names.
	 */
	preferSidecar?: SidecarPluginName[];
	/** MDX output format. `.md` files ignore this. */
	outputFormat?: "function-body" | "module";
	/** Whitespace-collapse the MDX body. `.md` files ignore this. */
	minify?: boolean;
}

export interface UserConfig<
	C extends Record<string, CollectionConfig<unknown>> = Record<string, CollectionConfig<unknown>>,
> {
	root?: string;
	strict?: boolean;
	output?: OutputOptions;
	collections: C;
	loaders?: CustomLoader[];
	/** Content-pipeline config. One block, applied to both `.md` and `.mdx`. */
	content?: ContentOptions;
	prepare?: (
		data: Record<string, unknown[]>,
		ctx: { config: UserConfig<C> },
	) => unknown;
	complete?: (
		data: Record<string, unknown[]>,
		ctx: { config: UserConfig<C> },
	) => unknown;
}

export interface BuildCollectionReport {
	name: string;
	records: number;
	outputPath: string;
}

export interface BuildErrorReport {
	file: string;
	message: string;
}

/**
 * One diagnostic emitted by the native engine. Stable shape so
 * consumers can pretty-print one-by-one with their own colors.
 */
export interface DiagnosticReport {
	/** Stable error code, e.g. `T007`, `TW005`, `E001`. */
	code: string;
	/** `bug | error | warning | help | note`. */
	severity: "bug" | "error" | "warning" | "help" | "note";
	/** Human-readable summary line. */
	message: string;
	/** Optional follow-up hint (e.g. `bundled themes: …`). */
	help?: string;
	/** First label's source-file path. */
	file?: string;
	/** First label's 1-based line. */
	line?: number;
	/** First label's 1-based column. */
	column?: number;
}

export interface BuildReport {
	diagnostics: DiagnosticReport[];
	collections: BuildCollectionReport[];
	errors: BuildErrorReport[];
}

interface NativeCollectionInput {
	name: string;
	pattern: string;
	baseDir: string;
	schema?: SchemaDescriptor | null;
	single?: boolean;
}

interface NativeBuildInput {
	outputDir: string;
	collections: NativeCollectionInput[];
	root?: string;
	strict?: boolean;
	clean?: boolean;
	outputAssets?: string | null;
	outputBase?: string | null;
	outputName?: string | null;
	outputFormat?: string | null;
	markdownRemarkPlugins?: Pluggable[];
	markdownRehypePlugins?: Pluggable[];
	mdxRemarkPlugins?: Pluggable[];
	mdxRehypePlugins?: Pluggable[];
	copyLinkedFiles?: boolean;
	mdxOutputFormat?: string;
	mdxMinify?: boolean;
	markdownGfm?: boolean;
	includeHtml?: boolean;
	forceSidecar?: boolean;
	preferSidecar?: SidecarPluginName[];
	mermaid?: MermaidOptions;
	prettyCode?: PrettyCodeOptions;
}

/**
 * Velite-shaped context object passed as the second argument to
 * `s.<...>().transform((data, ctx) => ...)` callbacks. The fields mirror
 * what velite's transform signature exposes so existing
 * `(data, { path, meta }) => ...` config blocks port over with a
 * one-line import swap. `meta` is a velite-compat alias of `path`.
 */
export interface TransformCtx {
	/** Absolute path of the source file that produced this record. */
	path: string;
	/** Velite-compat alias: `{ path }`. */
	meta: { path: string };
	/** The collection name this record belongs to. */
	collection: string;
}

type CbFn = (v: unknown, ctx: TransformCtx) => unknown;

// The callback registry has to survive across separate module instances
// of `@gentleduck/md`, because module loaders like tsx will load this
// file twice in the same process — once when the user's config imports
// `s` to register transforms, and once when the runner imports `build()`
// to consume them. Two `new Map()` instances at module top-level would
// give us two disjoint registries; the schema produced by the config
// would carry callback ids that the runner's registry knows nothing
// about, and every transform would silently be skipped. Stashing the
// shared state on `globalThis` is the standard JS workaround.
interface DmcRegistryHolder {
	__dmcRegistry__?: { map: Map<number, CbFn>; nextId: number };
}
const __holder = globalThis as unknown as DmcRegistryHolder;
__holder.__dmcRegistry__ ??= { map: new Map<number, CbFn>(), nextId: 0 };
const cbRegistry = __holder.__dmcRegistry__.map;
const registerCallback = (fn: CbFn): number => {
	const reg = (globalThis as unknown as DmcRegistryHolder).__dmcRegistry__!;
	const id = ++reg.nextId;
	reg.map.set(id, fn);
	return id;
};

/**
 * Standard Schema v1 issue surface (https://standardschema.dev). Used by
 * downstream tooling (form libraries, type generators, etc) that accept
 * any standard-schema-compliant validator.
 */
export interface StandardSchemaV1Issue {
	readonly message: string;
	readonly path?: ReadonlyArray<PropertyKey | { readonly key: PropertyKey }>;
}

/** Standard Schema v1 result. */
export type StandardSchemaV1Result<Output> =
	| { readonly value: Output; readonly issues?: undefined }
	| { readonly issues: ReadonlyArray<StandardSchemaV1Issue>; readonly value?: undefined };

/** Standard Schema v1 props block (the `~standard` field). */
export interface StandardSchemaV1Props<Input = unknown, Output = Input> {
	readonly version: 1;
	readonly vendor: string;
	readonly validate: (
		value: unknown,
	) => StandardSchemaV1Result<Output> | Promise<StandardSchemaV1Result<Output>>;
	/**
	 * Phantom type holder. Always present at the type level so consumers
	 * can read `schema['~standard']['types']['output']` directly without
	 * a `NonNullable` indirection. Never set at runtime.
	 */
	readonly types: { readonly input: Input; readonly output: Output };
}

export class SchemaBuilder<_T = unknown> {
	[k: string]: unknown;
	/**
	 * Phantom type: never set at runtime, used by `_output`-style type
	 * inference (velite-compat). Prefer the Standard Schema path:
	 * `schema['~standard']['types']['output']`.
	 */
	declare readonly _output: _T;
	/**
	 * Standard Schema v1 (https://standardschema.dev) compliance block.
	 * Lets the schema be consumed by any tooling that accepts
	 * standard-schema validators (zod, valibot, arktype, etc).
	 *
	 * `validate` is a pass-through: actual validation runs server-side
	 * inside the napi engine during `build()`. Calling `validate` here
	 * does not exercise the rust validators — it only satisfies the
	 * standard-schema interface for tools that probe it.
	 */
	get '~standard'(): StandardSchemaV1Props<_T, _T> {
		return {
			version: 1,
			vendor: 'gentleduck-md',
			validate: (value) => ({ value: value as _T }),
			// `types` is a phantom for static type lookup; never read at
			// runtime. Cast keeps the runtime object minimal.
			types: undefined as unknown as { readonly input: _T; readonly output: _T },
		};
	}
	constructor(descriptor: SchemaDescriptor) {
		Object.assign(this, descriptor);
	}
	toJSON(): SchemaDescriptor {
		const out: SchemaDescriptor = { kind: this.kind as SchemaKind };
		for (const k of Object.keys(this)) out[k] = this[k];
		return out;
	}
	// Fluent helpers preserve the inferred output `_T` so chains like
	// `s.string().max(99)` stay typed as `SchemaBuilder<string>` instead
	// of decaying to `SchemaBuilder<unknown>`. Consumers reading
	// `data.title` inside `.transform((data) => …)` then see `string`.
	optional(): SchemaBuilder<_T | undefined> {
		return new SchemaBuilder<_T | undefined>({ kind: "optional", inner: this.toJSON() });
	}
	nullable(): SchemaBuilder<_T | null> {
		return new SchemaBuilder<_T | null>({ kind: "nullable", inner: this.toJSON() });
	}
	default(value: _T): SchemaBuilder<_T> {
		return new SchemaBuilder<_T>({
			kind: "default",
			inner: this.toJSON(),
			fallback: value,
		});
	}
	min(n: number): SchemaBuilder<_T> {
		return new SchemaBuilder<_T>({ ...this.toJSON(), min: n });
	}
	max(n: number): SchemaBuilder<_T> {
		return new SchemaBuilder<_T>({ ...this.toJSON(), max: n });
	}
	length(n: number): SchemaBuilder<_T> {
		return new SchemaBuilder<_T>({ ...this.toJSON(), length: n });
	}
	regex(p: string): SchemaBuilder<_T> {
		return new SchemaBuilder<_T>({ ...this.toJSON(), regex: p });
	}
	int(): SchemaBuilder<_T> {
		return new SchemaBuilder<_T>({ ...this.toJSON(), int: true });
	}
	by(bucket: string): SchemaBuilder<_T> {
		return new SchemaBuilder<_T>({ ...this.toJSON(), bucket });
	}
	reserved(list: string[]): SchemaBuilder<_T> {
		return new SchemaBuilder<_T>({ ...this.toJSON(), reserved: list });
	}
	passthrough(): SchemaBuilder<_T> {
		return new SchemaBuilder<_T>({ ...this.toJSON(), passthrough: true });
	}
	transform<R>(
		fn: (value: _T, ctx: TransformCtx) => R,
	): SchemaBuilder<Awaited<R>> {
		return new SchemaBuilder<Awaited<R>>({
			kind: "transform",
			inner: this.toJSON(),
			__callbackId: registerCallback(fn as CbFn),
		});
	}
	refine(
		fn: (value: _T, ctx: TransformCtx) => boolean,
		message?: string,
	): SchemaBuilder<_T> {
		return new SchemaBuilder<_T>({
			kind: "refine",
			inner: this.toJSON(),
			__callbackId: registerCallback(fn as CbFn),
			__message: message,
		});
	}
}

const sb = <T = unknown>(d: SchemaDescriptor): SchemaBuilder<T> =>
	new SchemaBuilder<T>(d);

/**
 * Lift a record of `SchemaBuilder` fields into the inferred output type.
 * Used by `s.object({...})` so `.transform((data, ctx) => ...)` callbacks
 * see `data` as the schema's actual shape instead of `unknown`.
 */
export type InferObject<S extends Record<string, SchemaBuilder>> = {
	[K in keyof S]: S[K] extends SchemaBuilder<infer T> ? T : never;
};

export interface SBuilders {
	string(): SchemaBuilder<string>;
	number(): SchemaBuilder<number>;
	boolean(): SchemaBuilder<boolean>;
	array<I>(item: SchemaBuilder<I>): SchemaBuilder<I[]>;
	object<S extends Record<string, SchemaBuilder>>(
		fields: S,
	): SchemaBuilder<InferObject<S>>;
	record<V>(value: SchemaBuilder<V>): SchemaBuilder<Record<string, V>>;
	tuple(items: SchemaBuilder[]): SchemaBuilder<unknown[]>;
	intersection<A, B>(
		a: SchemaBuilder<A>,
		b: SchemaBuilder<B>,
	): SchemaBuilder<A & B>;
	enum<T>(variants: T[]): SchemaBuilder<T>;
	literal<T>(value: T): SchemaBuilder<T>;
	union<T>(variants: SchemaBuilder<T>[]): SchemaBuilder<T>;
	discriminatedUnion<T>(
		discriminator: string,
		variants: SchemaBuilder<T>[],
	): SchemaBuilder<T>;
	coerce: {
		string(): SchemaBuilder<string>;
		number(): SchemaBuilder<number>;
		boolean(): SchemaBuilder<boolean>;
		date(): SchemaBuilder<string>;
	};
	raw(): SchemaBuilder<string>;
	markdown(): SchemaBuilder<string>;
	mdx(): SchemaBuilder<string>;
	toc(): SchemaBuilder<TocItem[]>;
	metadata(): SchemaBuilder<Metadata>;
	excerpt(opts?: { length?: number }): SchemaBuilder<string>;
	path(opts?: { removeIndex?: boolean }): SchemaBuilder<string>;
	slug(bucket?: string, reserved?: string[]): SchemaBuilder<string>;
	unique(bucket?: string): SchemaBuilder<string>;
	isodate(): SchemaBuilder<string>;
	file(opts?: { allowNonRelativePath?: boolean }): SchemaBuilder<string>;
	image(opts?: {
		absoluteRoot?: string;
	}): SchemaBuilder<{ src: string; width: number; height: number }>;
}

export const s: SBuilders = {
	string: () => sb({ kind: "string" }),
	number: () => sb({ kind: "number" }),
	boolean: () => sb({ kind: "boolean" }),
	array: (item) =>
		sb({ kind: "array", item: (item as SchemaBuilder).toJSON() }),
	object: (fields) =>
		sb({
			kind: "object",
			fields: Object.fromEntries(
				Object.entries(fields).map(([k, v]) => [
					k,
					(v as SchemaBuilder).toJSON(),
				]),
			),
		}),
	record: (value) =>
		sb({ kind: "record", value: (value as SchemaBuilder).toJSON() }),
	tuple: (items) =>
		sb({
			kind: "tuple",
			items: items.map((v) => (v as SchemaBuilder).toJSON()),
		}),
	intersection: (a, b) =>
		sb({
			kind: "intersection",
			left: (a as SchemaBuilder).toJSON(),
			right: (b as SchemaBuilder).toJSON(),
		}),
	enum: (variants) => sb({ kind: "enum", variants }),
	literal: (expected) => sb({ kind: "literal", expected }),
	union: (variants) =>
		sb({
			kind: "union",
			variants: variants.map((v) => (v as SchemaBuilder).toJSON()),
		}),
	discriminatedUnion: (discriminator, variants) =>
		sb({
			kind: "discriminatedUnion",
			discriminator,
			variants: variants.map((v) => (v as SchemaBuilder).toJSON()),
		}),
	coerce: {
		string: () => sb({ kind: "coerce.string" }),
		number: () => sb({ kind: "coerce.number" }),
		boolean: () => sb({ kind: "coerce.boolean" }),
		date: () => sb({ kind: "coerce.date" }),
	},
	raw: () => sb<string>({ kind: "raw" }),
	markdown: () => sb<string>({ kind: "markdown" }),
	mdx: () => sb<string>({ kind: "mdx" }),
	toc: () => sb<TocItem[]>({ kind: "toc" }),
	metadata: () => sb<Metadata>({ kind: "metadata" }),
	excerpt: (opts = {}) => sb<string>({ kind: "excerpt", ...opts }),
	path: (opts = {}) => sb<string>({ kind: "path", ...opts }),
	slug: (bucket, reserved) =>
		sb<string>({ kind: "slug", bucket, reserved }),
	unique: (bucket) => sb<string>({ kind: "unique", bucket }),
	isodate: () => sb<string>({ kind: "isodate" }),
	file: (opts = {}) => sb<string>({ kind: "file", ...opts }),
	image: (opts = {}) =>
		sb<{ src: string; width: number; height: number }>({ kind: "image", ...opts }),
};

export const defineConfig = <C extends Record<string, CollectionConfig<unknown>>>(
	config: UserConfig<C>,
): UserConfig<C> => config;
export const defineCollection = <S>(
	c: CollectionConfig<S>,
): CollectionConfig<S> => c;
export const defineLoader = <L>(l: L): L => l;
export const defineSchema = <S>(sch: S): S => sch;

export interface CustomLoader<T = unknown> {
	test: RegExp | string;
	load: (file: { path: string; value: string }) => T | Promise<T>;
}

export async function applyLoaders<T>(
	loaders: CustomLoader<T>[] | undefined,
	filePath: string,
	content: string,
): Promise<T | null> {
	if (!loaders || loaders.length === 0) return null;
	for (const loader of loaders) {
		const re =
			loader.test instanceof RegExp ? loader.test : new RegExp(loader.test);
		if (re.test(filePath)) {
			return await loader.load({ path: filePath, value: content });
		}
	}
	return null;
}

interface PendingCallback {
	path: string[];
	kind: "transform" | "refine";
	fn: CbFn;
	message?: string;
}

function collectCallbacks(
	descriptor: SchemaDescriptor | undefined,
	base: string[] = [],
): PendingCallback[] {
	if (!descriptor || typeof descriptor !== "object") return [];
	const found: PendingCallback[] = [];
	if (
		descriptor.kind === "transform" &&
		typeof descriptor.__callbackId === "number"
	) {
		const fn = cbRegistry.get(descriptor.__callbackId as number);
		if (fn) found.push({ path: [...base], kind: "transform", fn });
	}
	if (
		descriptor.kind === "refine" &&
		typeof descriptor.__callbackId === "number"
	) {
		const fn = cbRegistry.get(descriptor.__callbackId as number);
		if (fn)
			found.push({
				path: [...base],
				kind: "refine",
				fn,
				message: descriptor.__message as string | undefined,
			});
	}
	if (descriptor.inner)
		found.push(...collectCallbacks(descriptor.inner as SchemaDescriptor, base));
	if (descriptor.kind === "object" && descriptor.fields) {
		for (const [k, v] of Object.entries(
			descriptor.fields as Record<string, SchemaDescriptor>,
		)) {
			found.push(...collectCallbacks(v, [...base, k]));
		}
	}
	if (descriptor.kind === "array" && descriptor.item) {
		found.push(
			...collectCallbacks(descriptor.item as SchemaDescriptor, [...base, "*"]),
		);
	}
	return found;
}

interface PathTarget {
	parent: Record<string, unknown>;
	key: string;
}

function walkPath(obj: unknown, path: string[]): PathTarget[] {
	if (path.length === 0) return [];
	if (path[0] === "*") {
		if (!Array.isArray(obj)) return [];
		return obj.flatMap((_, i) =>
			walkPath((obj as unknown[])[i], path.slice(1)),
		);
	}
	const [key, ...rest] = path;
	if (
		obj == null ||
		typeof obj !== "object" ||
		!(key in (obj as Record<string, unknown>))
	)
		return [];
	if (rest.length === 0)
		return [{ parent: obj as Record<string, unknown>, key }];
	return walkPath((obj as Record<string, unknown>)[key], rest);
}

function applyCallbacks(
	record: unknown,
	cbs: PendingCallback[],
	errors: BuildErrorReport[],
	file: string,
	ctx: TransformCtx,
): void {
	for (const cb of cbs) {
		// Root-level transform / refine: the descriptor is at the top of
		// the schema (e.g. `s.object({...}).transform(fn)`). walkPath
		// with an empty path returns nothing, so handle that case here
		// by mutating the record in place from the callback's return.
		if (cb.path.length === 0) {
			if (cb.kind === "transform") {
				try {
					const next = cb.fn(record, ctx);
					if (
						record &&
						typeof record === "object" &&
						next &&
						typeof next === "object"
					) {
						const r = record as Record<string, unknown>;
						for (const k of Object.keys(r)) delete r[k];
						Object.assign(r, next as Record<string, unknown>);
					}
				} catch (e) {
					errors.push({
						file,
						message: `transform threw: ${(e as Error).message ?? e}`,
					});
				}
			} else {
				try {
					if (!cb.fn(record, ctx)) {
						errors.push({
							file,
							message: cb.message ?? "refine failed",
						});
					}
				} catch (e) {
					errors.push({
						file,
						message: `refine threw: ${(e as Error).message ?? e}`,
					});
				}
			}
			continue;
		}
		for (const { parent, key } of walkPath(record, cb.path)) {
			const v = parent[key];
			if (cb.kind === "transform") {
				try {
					parent[key] = cb.fn(v, ctx);
				} catch (e) {
					errors.push({
						file,
						message: `${cb.path.join(".")}: transform threw: ${(e as Error).message ?? e}`,
					});
				}
			} else {
				let ok = false;
				try {
					ok = !!cb.fn(v, ctx);
				} catch (e) {
					errors.push({
						file,
						message: `${cb.path.join(".")}: refine threw: ${(e as Error).message ?? e}`,
					});
					continue;
				}
				if (!ok)
					errors.push({
						file,
						message: `${cb.path.join(".")}: ${cb.message ?? "failed refinement"}`,
					});
			}
		}
	}
}

function adaptToBuildInput(
	input: UserConfig | NativeBuildInput,
): NativeBuildInput {
	if ("outputDir" in input && Array.isArray(input.collections))
		return input as NativeBuildInput;
	const cfg = input as UserConfig;
	const root = cfg.root ?? ".";
	const outputDir = cfg.output?.data ?? ".gentleduck";
	const collections: NativeCollectionInput[] = Object.entries(
		cfg.collections ?? {},
	).map(([key, c]) => ({
		name: c.name ?? key,
		pattern: Array.isArray(c.pattern) ? c.pattern[0] : c.pattern,
		baseDir: c.baseDir ?? root,
		schema:
			c.schema instanceof SchemaBuilder
				? c.schema.toJSON()
				: (c.schema as SchemaDescriptor | undefined),
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
		// Single `content` block drives both `.md` and `.mdx` pipelines.
		// Plugin lists feed both markdown- and mdx-side slots so a config
		// without explicit per-extension overrides Just Works.
		markdownRemarkPlugins: cfg.content?.remarkPlugins,
		markdownRehypePlugins: cfg.content?.rehypePlugins,
		mdxRemarkPlugins: cfg.content?.remarkPlugins,
		mdxRehypePlugins: cfg.content?.rehypePlugins,
		copyLinkedFiles: cfg.content?.copyLinkedFiles,
		mdxOutputFormat: cfg.content?.outputFormat,
		mdxMinify: cfg.content?.minify,
		markdownGfm: cfg.content?.gfm,
		includeHtml: (cfg.output as { html?: boolean } | undefined)?.html,
		forceSidecar: cfg.content?.forceSidecar,
		preferSidecar: cfg.content?.preferSidecar,
		mermaid: cfg.content?.mermaid,
		prettyCode: cfg.content?.prettyCode,
	};
}

export function compile(source: string): CompileOutput {
	return native.compile(source) as CompileOutput;
}

export function compileMany(sources: string[]): CompileOutput[] {
	return native.compileMany(sources) as CompileOutput[];
}

/**
 * Run user `preMdxPlugins` on each `.mdx` / `.md` source file via
 * `unified(remark-parse + remark-mdx + plugins + remark-stringify)`.
 * Writes processed copies into `<output>/.cache/preprocessed/<rel>`
 * and returns the mirror root path. Native build reads from there.
 *
 * Cache layout: every dmc-owned on-disk cache lives under one root
 * (`<output>/.cache/`):
 *
 *   <output>/.cache/
 *     ├── preprocessed/        (preMdx mirror + per-file manifest)
 *     ├── dmc/                 (native per-file compile records)
 *     └── math.json            (KaTeX/MathML render cache)
 *
 * One root means one `.gitignore` entry, one `rm -rf` to nuke
 * everything for a clean rebuild, and no cache files leaking into
 * the source tree (the previous `<root>/.dmc-cache/` location).
 */
async function preprocessMdxIntoMirror(
	input: UserConfig,
	plugins: PreMdxPlugin[],
): Promise<string> {
	const { mkdirSync, writeFileSync, readFileSync, existsSync, rmSync } = await import("node:fs");
	const { join, relative, dirname, resolve } = await import("node:path");
	const { createHash } = await import("node:crypto");
	const t0 = performance.now();
	const root = resolve(input.root ?? ".");
	const outputDir = resolve(input.output?.data ?? ".gentleduck");
	const mirrorRoot = join(outputDir, ".cache", "preprocessed");
	const manifestPath = join(mirrorRoot, ".manifest.json");

	// Sweep legacy mirror locations: `<root>/.dmc-cache/` and
	// `<output>/.dmc-cache/`. Current location is `<output>/.cache/preprocessed/`.
	for (const legacy of [join(root, ".dmc-cache"), join(outputDir, ".dmc-cache")]) {
		if (existsSync(legacy) && resolve(legacy) !== resolve(join(outputDir, ".cache"))) {
			try {
				rmSync(legacy, { recursive: true, force: true });
			} catch {
				// best-effort
			}
		}
	}

	mkdirSync(mirrorRoot, { recursive: true });
	let hits = 0;
	let misses = 0;

	const sources = walkDir(root).filter(
		(f) =>
			(f.endsWith(".mdx") || f.endsWith(".md")) &&
			!f.includes(".dmc-cache") &&
			!f.includes("node_modules") &&
			!f.includes(".gentleduck"),
	);

	const sha = (s: string | Buffer): string => createHash("sha256").update(s).digest("hex");

	// Plugins: hash the function body + module path of each entry
	// once per build. Captures source mutations to the plugin itself
	// (e.g. editing `rehype-component.ts`) without dragging in a
	// graph-wide module hasher. Falls back to the function `.toString()`
	// when the plugin is an inline arrow.
	const pluginsHash = sha(
		plugins
			.map((p) => {
				// biome-ignore lint/suspicious/noExplicitAny: plugin shape varies
				const fn: any = Array.isArray(p) ? p[0] : p;
				const opts = Array.isArray(p) ? p.slice(1) : [];
				const body = typeof fn === "function" ? fn.toString() : String(fn);
				return body + "::" + JSON.stringify(opts);
			})
			.join("\0"),
	);

	// User-declared extra inputs — files outside the source `.mdx` whose
	// content gates the cache (e.g. `__ui_registry__/index.ts` read by
	// `rehypeComponent`). Missing files contribute the empty hash, so a
	// fresh checkout doesn't error.
	const extraInputs = input.content?.preMdxCacheInputs ?? [];
	const extraInputsHash = sha(
		extraInputs
			.map((p) => {
				const abs = resolve(p);
				try {
					return abs + "::" + sha(readFileSync(abs));
				} catch {
					return abs + "::missing";
				}
			})
			.join("\0"),
	);

	type ManifestEntry = { sourceHash: string; pluginsHash: string; extraInputsHash: string };
	type Manifest = Record<string, ManifestEntry>;
	let prev: Manifest = {};
	if (existsSync(manifestPath)) {
		try {
			prev = JSON.parse(readFileSync(manifestPath, "utf8")) as Manifest;
		} catch {
			prev = {};
		}
	}
	const next: Manifest = {};
	const validRels = new Set<string>();

	const { unified } = await import("unified");
	const { default: remarkParse } = await import("remark-parse");
	const { default: remarkMdx } = await import("remark-mdx");
	const { default: remarkFrontmatter } = await import("remark-frontmatter");
	const { default: remarkStringify } = await import("remark-stringify");
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const apply = (proc: any, p: Pluggable) => {
		if (typeof p === "function") return proc.use(p);
		if (Array.isArray(p)) return proc.use(p[0] as Plugin, p[1]);
		return proc.use(p);
	};

	for (const abs of sources) {
		const rel = relative(root, abs);
		const mirrorAbs = join(mirrorRoot, rel);
		const source = readFileSync(abs, "utf8");
		const sourceHash = sha(source);
		validRels.add(rel);

		// Cache hit: source bytes, plugin set, AND every declared extra
		// input identical to the last build. Trust the on-disk mirror
		// and skip the unified pipeline for this file.
		const cached = prev[rel];
		if (
			cached &&
			cached.sourceHash === sourceHash &&
			cached.pluginsHash === pluginsHash &&
			cached.extraInputsHash === extraInputsHash &&
			existsSync(mirrorAbs)
		) {
			next[rel] = cached;
			hits++;
			continue;
		}
		misses++;

		// `remark-frontmatter` preserves YAML/TOML frontmatter as a
		// dedicated mdast `yaml` / `toml` node so `remark-stringify`
		// outputs `---\n…\n---` verbatim instead of mangling it into
		// a thematic break.
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		let proc: any = unified()
			.use(remarkParse)
			.use(remarkFrontmatter, ["yaml", "toml"])
			.use(remarkMdx);
		for (const p of plugins) proc = apply(proc, p);
		proc = proc.use(remarkStringify);
		try {
			const processed = String(await proc.process(source));
			const dedented = dedentJsxFlowChildren(processed);
			mkdirSync(dirname(mirrorAbs), { recursive: true });
			writeFileSync(mirrorAbs, dedented);
			next[rel] = { sourceHash, pluginsHash, extraInputsHash };
		} catch {
			// Plugin failure → fall back to original source so the
			// build doesn't lose the file. Diagnostics surface via the
			// usual error reporting once native parse runs.
			mkdirSync(dirname(mirrorAbs), { recursive: true });
			writeFileSync(mirrorAbs, source);
			// Don't cache a fall-back as if it were a successful run —
			// a transient plugin error should be retried next build.
			delete next[rel];
		}
	}

	// Sweep mirror entries whose source no longer exists. Without
	// this, deleting an `.mdx` from `content/docs/**` would leave a
	// stale mirror file behind that the native build picks up.
	for (const rel of Object.keys(prev)) {
		if (validRels.has(rel)) continue;
		const stale = join(mirrorRoot, rel);
		if (existsSync(stale)) {
			try {
				rmSync(stale);
			} catch {
				// Best-effort: if the file is locked, skip — the next
				// successful build will catch it.
			}
		}
	}

	try {
		writeFileSync(manifestPath, JSON.stringify(next));
	} catch {
		// Manifest write failure degrades gracefully to "no cache" on
		// the next build. Don't fail the build over it.
	}

	const elapsed = (performance.now() - t0).toFixed(0);
	console.log(`[duck-md preMdx] ${sources.length} files: ${hits} hits, ${misses} misses (${elapsed}ms)`);

	return mirrorRoot;
}

/**
 * Remove the 2-space indent that `mdast-util-mdx-jsx` injects on flow
 * JSX-element children. We target only the OPEN-CLOSE JSX block shape
 * (e.g. `<Tag …>\n  body\n</Tag>`) so unrelated indentation elsewhere
 * is untouched. dmc's parser then sees fenced code blocks at column
 * zero relative to the JSX block and recognises them as proper fences.
 */
/**
 * `mdast-util-mdx-jsx` indents flow JSX-element children by 2 spaces
 * per nesting level. Walk the source line-by-line tracking
 * (a) fence state and (b) JSX nesting depth; strip `depth * 2`
 * leading spaces from every line. Tags inside fenced code blocks are
 * literal text, not real JSX, so depth stays unchanged while in fence.
 */
function dedentJsxFlowChildren(src: string): string {
	const lines = src.split("\n");
	const out: string[] = new Array(lines.length);
	let depth = 0;
	let inFence = false;
	let fenceMarker = "";

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i];
		const trimmed = line.replace(/^\s+/, "");

		if (!inFence) {
			const open = trimmed.match(/^(`{3,}|~{3,})/);
			if (open) {
				out[i] = stripUpTo(line, depth * 2);
				inFence = true;
				fenceMarker = open[1];
				continue;
			}
		} else {
			const close = trimmed.match(/^(`{3,}|~{3,})\s*$/);
			if (close && close[1][0] === fenceMarker[0] && close[1].length >= fenceMarker.length) {
				out[i] = stripUpTo(line, depth * 2);
				inFence = false;
				fenceMarker = "";
				continue;
			}
		}

		if (inFence) {
			out[i] = stripUpTo(line, depth * 2);
			continue;
		}

		// Match a JSX tag at line start. mdast-mdx-jsx indents children
		// of every flow JSX element — Capitalised (`<LinkedCard>`) AND
		// lowercase host tags (`<div>`, `<svg>`, `<p>`) — by 2 spaces
		// per nesting level. The dedent walker has to bump depth on
		// either, otherwise lowercase wrappers like `<svg>` leave a
		// 4-space residue on their children, which CommonMark then
		// re-classifies as an indented code block (so `<title>` and
		// `<path>` inside an SVG get pretty-coded as plaintext).
		const openMatch = trimmed.match(/^<([A-Za-z][\w-]*)\b[^\n]*?(\/?)>/);
		const closeOnlyMatch = /^<\/([A-Za-z][\w-]*)>\s*$/.exec(trimmed);

		if (closeOnlyMatch && depth > 0) {
			depth--;
			out[i] = stripUpTo(line, depth * 2);
			continue;
		}
		out[i] = stripUpTo(line, depth * 2);
		if (openMatch && openMatch[2] !== "/") {
			const tag = openMatch[1];
			// `<Tag>…</Tag>` on the same line is balanced — don't bump depth.
			const balanced = new RegExp(`</${tag}>\\s*$`).test(trimmed);
			if (!balanced) depth++;
		}
	}

	return out.join("\n");
}

/** Remove up to `n` leading spaces from `line`. */
function stripUpTo(line: string, n: number): string {
	let i = 0;
	while (i < n && i < line.length && line.charAt(i) === " ") i++;
	return line.slice(i);
}

function rewireMirrorPaths(report: BuildReport, originalRoot: string, mirrorRoot: string): void {
	const { resolve } = require("node:path") as typeof import("node:path");
	const orig = resolve(originalRoot);
	const mir = resolve(mirrorRoot);
	const remap = (s: string): string => (s.startsWith(mir) ? orig + s.slice(mir.length) : s);
	for (const c of report.collections) {
		try {
			const records: Record<string, unknown>[] = JSON.parse(readFileSync(c.outputPath, "utf8"));
			for (const r of records) {
				for (const k of ["sourceFilePath", "path"] as const) {
					const v = r[k];
					if (typeof v === "string") r[k] = remap(v);
				}
			}
			writeFileSync(c.outputPath, JSON.stringify(records, null, 2));
		} catch {
			// best-effort; collection JSON may not exist on partial failures
		}
	}
	// Diagnostic spans carry the mirror path AND mirror line/column.
	// Mirror line numbers don't match the original source (preMdx
	// stringify reflows whitespace and JSX wrappers), so remapping
	// the path alone would point users at the wrong line. Keep the
	// mirror path verbatim so `path:line:col` stays consistent.
	// Consumers who want to follow the path open the file under
	// `.dmc-cache/preprocessed/...`.
}

async function processWithUnified(
	markdown: string,
	remarkPlugins: Pluggable[],
	rehypePlugins: Pluggable[],
): Promise<string> {
	const { unified } = await import("unified");
	const { default: remarkParse } = await import("remark-parse");
	const { default: remarkMdx } = await import("remark-mdx");
	const { default: remarkRehype } = await import("remark-rehype");
	const { default: rehypeStringify } = await import("rehype-stringify");

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const apply = (proc: any, p: Pluggable) => {
		if (typeof p === "function") return proc.use(p);
		if (Array.isArray(p)) return proc.use(p[0] as Plugin, p[1]);
		return proc.use(p);
	};
	// `remark-mdx` makes JSX nodes (`<ComponentSource path="…" />`,
	// `<ComponentPreview name="…" />`, expressions, MDX import/export)
	// parse as proper mdast JSX — `mdxJsxFlowElement` /
	// `mdxJsxTextElement` with `name` + `attributes`. The
	// `passThrough` option on `remark-rehype` forwards them into hast
	// untouched so user rehype plugins can visit + mutate them.
	//
	// `rehype-raw` is intentionally NOT used: it re-parses raw HTML and
	// chokes on `mdxJsxFlowElement` nodes. With remark-mdx every JSX is
	// already a typed node, so no raw-HTML round-trip is needed.
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let proc: any = unified().use(remarkParse).use(remarkMdx);
	for (const p of remarkPlugins) proc = apply(proc, p);
	proc = proc.use(remarkRehype, {
		allowDangerousHtml: true,
		passThrough: [
			"mdxJsxFlowElement",
			"mdxJsxTextElement",
			"mdxFlowExpression",
			"mdxTextExpression",
			"mdxjsEsm",
		],
	});
	for (const p of rehypePlugins) proc = apply(proc, p);
	// Final pass: convert any remaining MDX JSX nodes to plain hast
	// elements so `rehype-stringify` can serialize them. User plugins
	// that read `node.name === 'ComponentSource'` etc still see them
	// upstream; this only fires for nodes left untouched.
	proc = proc.use(rehypeMdxJsxToHast).use(rehypeStringify, { allowDangerousHtml: true });

	const file = await proc.process(markdown);
	return String(file);
}

/**
 * Lower mdxJsxFlowElement / mdxJsxTextElement to plain hast `element`
 * nodes (lowercased tagName + attributes flattened into properties).
 * mdxFlowExpression / mdxTextExpression / mdxjsEsm get dropped — they
 * only make sense inside an MDX runtime, not in stringified HTML.
 */
function rehypeMdxJsxToHast() {
	type MdxJsxAttr = {
		type: "mdxJsxAttribute";
		name: string;
		value?: unknown;
	};
	type MdxNode = {
		type: string;
		name?: string;
		attributes?: MdxJsxAttr[];
		children?: unknown[];
		tagName?: string;
		properties?: Record<string, unknown>;
	};

	const lower = (node: MdxNode) => {
		const props: Record<string, unknown> = {};
		for (const a of node.attributes ?? []) {
			if (a.type !== "mdxJsxAttribute") continue;
			props[a.name] =
				typeof a.value === "string" || typeof a.value === "number" || typeof a.value === "boolean"
					? a.value
					: a.value == null
						? true
						: ((a.value as { value?: unknown }).value ?? "");
		}
		node.type = "element";
		node.tagName = node.name ?? "div";
		node.properties = props;
		delete node.name;
		delete node.attributes;
	};

	return (tree: unknown) => {
		const walk = (node: unknown) => {
			if (!node || typeof node !== "object") return;
			const n = node as MdxNode & { children?: MdxNode[] };
			if (n.type === "mdxJsxFlowElement" || n.type === "mdxJsxTextElement") {
				lower(n);
			} else if (n.type === "mdxFlowExpression" || n.type === "mdxTextExpression" || n.type === "mdxjsEsm") {
				n.type = "text";
				(n as unknown as { value: string }).value = "";
				n.children = [];
			}
			if (Array.isArray(n.children)) for (const c of n.children) walk(c);
		};
		walk(tree);
	};
}

function walkDir(dir: string): string[] {
	const out: string[] = [];
	try {
		for (const name of readdirSync(dir)) {
			const full = join(dir, name);
			const st = statSync(full);
			if (st.isDirectory()) out.push(...walkDir(full));
			else out.push(full);
		}
	} catch {}
	return out;
}

async function applyCustomLoaders(
	input: UserConfig,
	report: BuildReport,
): Promise<{ extras: Map<string, unknown[]> }> {
	const extras = new Map<string, unknown[]>();
	const loaders = (input.loaders as CustomLoader[] | undefined) ?? [];
	if (loaders.length === 0) return { extras };
	const root = input.root ?? ".";
	for (const [key, c] of Object.entries(input.collections)) {
		const baseDir = c.baseDir ?? root;
		const files = walkDir(baseDir);
		const matched: unknown[] = [];
		const matchedPaths = new Set<string>();
		for (const file of files) {
			const rel = relative(baseDir, file);
			for (const loader of loaders) {
				const re =
					loader.test instanceof RegExp ? loader.test : new RegExp(loader.test);
				if (re.test(rel) || re.test(file)) {
					const content = readFileSync(file, "utf8");
					const data = await loader.load({ path: file, value: content });
					if (data && typeof data === "object") {
						const record = { ...(data as object), sourceFilePath: file };
						matched.push(record);
						matchedPaths.add(file);
					}
					break;
				}
			}
		}
		if (matched.length > 0) {
			const name = c.name ?? key;
			extras.set(name, matched);
			const target = report.collections.find((rc) => rc.name === name);
			if (target) {
				const existing: { sourceFilePath?: string }[] = JSON.parse(
					readFileSync(target.outputPath, "utf8"),
				);
				const filtered = existing.filter(
					(r) => !matchedPaths.has(r?.sourceFilePath ?? ""),
				);
				const merged = [...filtered, ...matched];
				writeFileSync(target.outputPath, JSON.stringify(merged, null, 2));
				target.records = merged.length;
			}
		}
	}
	return { extras };
}

export async function build(input: UserConfig): Promise<BuildReport> {
	const collectionCallbacks = new Map<string, PendingCallback[]>();
	if (input?.collections && !Array.isArray(input.collections)) {
		for (const [key, c] of Object.entries(input.collections)) {
			if (c.schema) {
				const desc =
					c.schema instanceof SchemaBuilder
						? c.schema.toJSON()
						: (c.schema as SchemaDescriptor);
				const cbs = collectCallbacks(desc);
				if (cbs.length) collectionCallbacks.set(c.name ?? key, cbs);
			}
		}
	}

	// Pre-MDX preprocessing: source-level remark plugins run BEFORE
	// dmc's native parse + transform. Mutated source is written to a
	// `.dmc-cache/preprocessed/` mirror dir; native build reads from
	// the mirror. After build, source-path fields in the per-record
	// JSON are rewritten back to original paths so consumers see
	// untouched paths.
	const preMdxPlugins = input.content?.preMdxPlugins ?? [];
	let mirrorRoot: string | null = null;
	if (preMdxPlugins.length) {
		mirrorRoot = await preprocessMdxIntoMirror(input, preMdxPlugins);
	}

	// Strip JS plugin function refs from the napi input - they can't cross
	// the FFI boundary. The in-process post-pass below applies them.
	const stripped: UserConfig = {
		...input,
		root: mirrorRoot ?? input.root,
		content: input.content
			? {
					...input.content,
					remarkPlugins: undefined,
					rehypePlugins: undefined,
					preMdxPlugins: undefined,
				}
			: undefined,
	};
	const report = native.build(adaptToBuildInput(stripped)) as BuildReport;
	if (mirrorRoot) {
		rewireMirrorPaths(report, input.root ?? ".", mirrorRoot);
	}
	await applyCustomLoaders(input, report);

	// In-process unified pipeline - type-safe plugin refs run here.
	const remark: Pluggable[] = (input.content?.remarkPlugins ?? []) as Pluggable[];
	const rehype: Pluggable[] = (input.content?.rehypePlugins ?? []) as Pluggable[];
	if (remark.length || rehype.length) {
		for (const c of report.collections) {
			const records: Record<string, unknown>[] = JSON.parse(
				readFileSync(c.outputPath, "utf8"),
			);
			for (const r of records) {
				const md = (r.content as string | undefined) ?? "";
				try {
					r.html = await processWithUnified(md, remark, rehype);
				} catch (e) {
					report.errors.push({
						file: (r.sourceFilePath as string) ?? c.outputPath,
						message: `unified pipeline: ${(e as Error).message ?? e}`,
					});
				}
			}
			writeFileSync(c.outputPath, JSON.stringify(records, null, 2));
		}
	}

	const needPostprocess =
		collectionCallbacks.size > 0 || input.prepare || input.complete;
	if (!needPostprocess) return report;

	const data: Record<string, unknown[]> = {};
	for (const c of report.collections) {
		data[c.name] = JSON.parse(readFileSync(c.outputPath, "utf8"));
	}

	for (const c of report.collections) {
		const cbs = collectionCallbacks.get(c.name);
		if (!cbs) continue;
		const records = Array.isArray(data[c.name]) ? data[c.name] : [data[c.name]];
		for (const record of records) {
			// Build a velite-compatible ctx for the per-record transform.
			// `path` comes from the engine-injected `sourceFilePath` field;
			// fall back to the collection output path so the callback always
			// receives a defined string instead of undefined.
			let sourcePath: string = c.outputPath;
			if (record && typeof record === "object") {
				const sfp = (record as Record<string, unknown>).sourceFilePath;
				if (typeof sfp === "string") sourcePath = sfp;
			}
			const ctx: TransformCtx = {
				path: sourcePath,
				meta: { path: sourcePath },
				collection: c.name,
			};
			applyCallbacks(record, cbs, report.errors, c.outputPath, ctx);
		}
	}

	if (input.prepare) {
		const ret = await input.prepare(data, { config: input });
		if (ret === false) {
			for (const c of report.collections)
				try {
					unlinkSync(c.outputPath);
				} catch {}
			return report;
		}
	}

	for (const c of report.collections) {
		writeFileSync(c.outputPath, JSON.stringify(data[c.name], null, 2));
	}

	if (input.complete) await input.complete(data, { config: input });
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
	applyLoaders,
	s,
	SchemaBuilder,
};
