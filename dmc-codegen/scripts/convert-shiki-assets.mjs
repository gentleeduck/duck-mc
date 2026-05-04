#!/usr/bin/env node
/**
 * Convert shiki's bundled JSON themes + grammars into the plist
 * formats syntect accepts:
 *   themes-json/*.json       -> themes/*.tmTheme       (TextMate plist XML)
 *   grammars-json/*.json     -> grammars/*.tmLanguage  (TextMate plist XML)
 *
 * Run:
 *   cd dmc-codegen
 *   npm i plist
 *   node scripts/convert-shiki-assets.mjs
 *
 * Re-run any time you bump the shiki bundle. Output is deterministic.
 *
 * Theme conversion:
 *   - VS Code "colors" object -> TextMate top-level settings entry
 *     (no "scope" key) carrying background / foreground / caret / etc.
 *   - VS Code "tokenColors" array -> per-scope entries with
 *     "scope" (string, comma-joined when array) and "settings" dict.
 *   - "fontStyle" passes through as-is (italic, bold, underline).
 *
 * Grammar conversion:
 *   - .tmLanguage.json schema and TextMate plist schema are isomorphic
 *     for the fields syntect cares about. Direct JS-object-to-plist
 *     serialization works. The `plist` package handles type tagging.
 *
 * Limitations / known divergences from shiki's runtime output:
 *   - Embedded language grammars (markdown code fences, Vue SFC,
 *     Astro front-matter) use shiki-specific "embeddedLangs" +
 *     "embeddedLangsLazy". syntect doesn't honor those; outer
 *     grammar still highlights, embedded segments fall back.
 *   - Semantic tokens (TS LSP-driven highlighting) aren't in tmTheme
 *     scope. shiki itself drops them in static rendering, so parity
 *     is fine here.
 *   - Some VS Code themes set "tokenColors[].name" for editor UI
 *     (status bar, command palette). Stripped — irrelevant for code.
 */

import { readdirSync, readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join, dirname, basename } from "node:path";
import { fileURLToPath } from "node:url";

let plistBuild;
try {
  const mod = await import("plist");
  // `plist` v3 exports { build, parse } as named exports.
  // Older versions used a default export with the same shape.
  plistBuild = mod.build || mod.default?.build;
  if (!plistBuild) throw new Error("no `build` export found in plist module");
} catch (e) {
  console.error(`missing dep. run: npm i plist (${e.message})`);
  process.exit(1);
}

const HERE = dirname(fileURLToPath(import.meta.url));
const ASSETS = join(HERE, "..", "assets");
const THEMES_IN = join(ASSETS, "themes-json");
const THEMES_OUT = join(ASSETS, "themes");
const GRAMMARS_IN = join(ASSETS, "grammars-json");
const GRAMMARS_OUT = join(ASSETS, "grammars");

mkdirSync(THEMES_OUT, { recursive: true });
mkdirSync(GRAMMARS_OUT, { recursive: true });

// -------------------------------------------------------------------- themes

/** VS Code editor color keys -> TextMate global setting keys. */
const COLOR_MAP = {
  "editor.background":          "background",
  "editor.foreground":          "foreground",
  "editorCursor.foreground":    "caret",
  "editor.lineHighlightBackground": "lineHighlight",
  "editor.selectionBackground": "selection",
  "editorWhitespace.foreground": "invisibles",
  "editor.findMatchHighlightBackground": "findHighlight",
};

function convertTheme(json) {
  const settings = [];

  // 1. Global-style entry from VS Code "colors" map.
  const globals = {};
  if (json.colors) {
    for (const [k, v] of Object.entries(json.colors)) {
      if (COLOR_MAP[k] && typeof v === "string") {
        globals[COLOR_MAP[k]] = normalizeColor(v);
      }
    }
  }
  if (Object.keys(globals).length > 0) {
    settings.push({ settings: globals });
  }

  // 2. Per-scope entries from "tokenColors".
  for (const tc of json.tokenColors || []) {
    if (!tc.settings) continue;
    const entry = { settings: pickFontSettings(tc.settings) };
    if (Object.keys(entry.settings).length === 0) continue;
    if (tc.scope) {
      entry.scope = Array.isArray(tc.scope) ? tc.scope.join(", ") : String(tc.scope);
    }
    if (tc.name) entry.name = String(tc.name);
    settings.push(entry);
  }

  return {
    name: json.name || "Unnamed Theme",
    settings,
  };
}

function pickFontSettings(s) {
  const out = {};
  if (s.foreground) out.foreground = normalizeColor(s.foreground);
  if (s.background) out.background = normalizeColor(s.background);
  if (s.fontStyle) out.fontStyle = String(s.fontStyle);
  return out;
}

function normalizeColor(c) {
  // VS Code allows #rgb, #rgba, #rrggbb, #rrggbbaa.
  // TextMate themes traditionally use #rrggbb (no alpha) or
  // #rrggbbaa. syntect handles both. Pass through.
  return c.startsWith("#") ? c : `#${c}`;
}

let themesOk = 0;
let themesFail = 0;
for (const file of readdirSync(THEMES_IN)) {
  if (!file.endsWith(".json")) continue;
  const inPath = join(THEMES_IN, file);
  const outPath = join(THEMES_OUT, basename(file, ".json") + ".tmTheme");
  try {
    const json = JSON.parse(readFileSync(inPath, "utf8"));
    const tm = convertTheme(json);
    writeFileSync(outPath, plistBuild(tm));
    themesOk++;
  } catch (e) {
    console.error(`theme  ${file}: ${e.message}`);
    themesFail++;
  }
}
console.log(`themes:    ${themesOk} ok, ${themesFail} failed`);

// ------------------------------------------------------------------ grammars

/**
 * .tmLanguage.json and TextMate plist .tmLanguage carry the same
 * tree shape: name, scopeName, fileTypes, patterns, repository, etc.
 * Stringify the JS object straight into plist; the library handles
 * type tagging.
 *
 * One pre-pass: drop fields syntect doesn't recognise to keep the
 * plist smaller and avoid parser complaints. shiki adds:
 *   - embeddedLangs[]       (shiki-specific, no syntect equivalent)
 *   - embeddedLangsLazy[]   (same)
 *   - injectionSelector     (handled by syntect via "injections")
 *   - aliases               (shiki UI hint, irrelevant)
 *   - displayName           (UI)
 *   - balancedBracketScopes (VS Code editor hint)
 *   - unbalancedBracketScopes
 */
const GRAMMAR_DROP = new Set([
  "embeddedLangs",
  "embeddedLangsLazy",
  "aliases",
  "displayName",
  "balancedBracketScopes",
  "unbalancedBracketScopes",
]);

function stripGrammarExtensions(g) {
  if (Array.isArray(g)) return g.map(stripGrammarExtensions);
  if (g && typeof g === "object") {
    const out = {};
    for (const [k, v] of Object.entries(g)) {
      if (GRAMMAR_DROP.has(k)) continue;
      out[k] = stripGrammarExtensions(v);
    }
    return out;
  }
  return g;
}

let grammarsOk = 0;
let grammarsFail = 0;
for (const file of readdirSync(GRAMMARS_IN)) {
  if (!file.endsWith(".json")) continue;
  const inPath = join(GRAMMARS_IN, file);
  const outPath = join(GRAMMARS_OUT, basename(file, ".json") + ".tmLanguage");
  try {
    const json = JSON.parse(readFileSync(inPath, "utf8"));
    const cleaned = stripGrammarExtensions(json);
    writeFileSync(outPath, plistBuild(cleaned));
    grammarsOk++;
  } catch (e) {
    console.error(`grammar ${file}: ${e.message}`);
    grammarsFail++;
  }
}
console.log(`grammars:  ${grammarsOk} ok, ${grammarsFail} failed`);

// --------------------------------------------------------------------- recap

console.log(`
themes  -> ${THEMES_OUT}
grammars -> ${GRAMMARS_OUT}
`);
