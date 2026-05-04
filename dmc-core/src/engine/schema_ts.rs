//! `dmc-schema` descriptor -> TypeScript type emitter. Drives the typed
//! interfaces in the generated `index.d.ts`. Mirrors the `kind`s accepted
//! by `dmc_schema::compile_descriptor`; unknown shapes fall back to
//! `unknown` so the build never fails on an unrecognised descriptor.

use serde_json::Value;

use crate::engine::utils::is_js_ident;

const INDENT: &str = "  ";

/// Descriptor -> TS type string at `indent` levels (0 = top of interface body).
pub fn schema_to_ts(v: &Value, indent: usize) -> String {
  let kind = match v.get("kind").and_then(|k| k.as_str()) {
    Some(k) => k,
    None => return "unknown".into(),
  };

  match kind {
    "string" | "isodate" | "path" | "slug" | "unique" | "file" | "image" | "raw" | "markdown" | "mdx" | "excerpt" => {
      "string".into()
    },
    "number" => "number".into(),
    "boolean" => "boolean".into(),
    "metadata" => "{ readingTime: number; wordCount: number }".into(),
    "toc" => "TocItem[]".into(),
    "array" => {
      let item = v.get("item").map(|i| schema_to_ts(i, indent)).unwrap_or_else(|| "unknown".into());
      format!("{item}[]")
    },
    "object" => render_object(v, indent),
    "record" => {
      let val = v.get("value").map(|i| schema_to_ts(i, indent)).unwrap_or_else(|| "unknown".into());
      format!("{{ [k: string]: {val} }}")
    },
    "tuple" => {
      let items: Vec<String> = v
        .get("items")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().map(|i| schema_to_ts(i, indent)).collect())
        .unwrap_or_default();
      format!("[{}]", items.join(", "))
    },
    "enum" => {
      let parts: Vec<String> = v
        .get("variants")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(literal_value).collect())
        .unwrap_or_default();
      if parts.is_empty() { "string".into() } else { parts.join(" | ") }
    },
    "literal" => v.get("expected").and_then(literal_value).unwrap_or_else(|| "unknown".into()),
    "union" => {
      let parts: Vec<String> = v
        .get("variants")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().map(|i| schema_to_ts(i, indent)).collect())
        .unwrap_or_default();
      if parts.is_empty() { "unknown".into() } else { parts.join(" | ") }
    },
    "discriminatedUnion" => {
      let parts: Vec<String> = v
        .get("variants")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().map(|i| schema_to_ts(i, indent)).collect())
        .unwrap_or_default();
      if parts.is_empty() { "unknown".into() } else { parts.join(" | ") }
    },
    "intersection" => {
      let l = v.get("left").map(|i| schema_to_ts(i, indent)).unwrap_or_else(|| "unknown".into());
      let r = v.get("right").map(|i| schema_to_ts(i, indent)).unwrap_or_else(|| "unknown".into());
      format!("{l} & {r}")
    },
    // Unwrap-and-forward kinds.
    "optional" | "default" | "transform" | "refine" | "superRefine" | "super_refine" => {
      v.get("inner").map(|i| schema_to_ts(i, indent)).unwrap_or_else(|| "unknown".into())
    },
    "nullable" => {
      let inner = v.get("inner").map(|i| schema_to_ts(i, indent)).unwrap_or_else(|| "unknown".into());
      format!("{inner} | null")
    },
    "coerce.string" => "string".into(),
    "coerce.number" => "number".into(),
    "coerce.boolean" => "boolean".into(),
    "coerce.date" => "Date".into(),
    _ => "unknown".into(),
  }
}

/// Render the top-level object body as a `{ ... }` block at indent 0.
pub fn schema_to_ts_object(v: &Value) -> String {
  render_object(v, 0)
}

fn render_object(v: &Value, indent: usize) -> String {
  let pad_outer = INDENT.repeat(indent);
  let pad_inner = INDENT.repeat(indent + 1);

  let fields = match v.get("fields").and_then(|f| f.as_object()) {
    Some(f) => f,
    None => return "{}".into(),
  };

  let mut out = String::from("{\n");
  for (key, sub) in fields {
    let optional = matches!(sub.get("kind").and_then(|k| k.as_str()), Some("optional") | Some("default"),);
    let opt = if optional { "?" } else { "" };
    let ty = schema_to_ts(sub, indent + 1);
    let safe_key = if is_js_ident(key) { key.clone() } else { format!("'{}'", key.replace('\'', "\\'")) };
    out.push_str(&format!("{pad_inner}{safe_key}{opt}: {ty}\n"));
  }
  let passthrough = v.get("passthrough").and_then(|b| b.as_bool()).unwrap_or(false);
  if passthrough {
    out.push_str(&format!("{pad_inner}[k: string]: unknown\n"));
  }
  out.push_str(&format!("{pad_outer}}}"));
  out
}

/// JSON literal (string/number/bool/null) -> TS literal type string.
fn literal_value(v: &Value) -> Option<String> {
  match v {
    Value::String(s) => Some(format!("'{}'", s.replace('\'', "\\'"))),
    Value::Number(n) => Some(n.to_string()),
    Value::Bool(b) => Some(b.to_string()),
    Value::Null => Some("null".into()),
    _ => None,
  }
}
