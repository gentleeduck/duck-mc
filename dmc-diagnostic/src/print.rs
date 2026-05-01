//! Smart diagnostic printer for multi-file engines.
//!
//! Rule:
//! - Diagnostic primary label points at a real file we can read → render with
//!   source snippet + carets (`format_all` style).
//! - Otherwise (synthetic span, missing file, glob/config error) → render
//!   compact (`format_compact`).
//!
//! Source files are read at most once per render via a tiny cache.

use std::collections::HashMap;
use std::io::IsTerminal;
use std::sync::Arc;

use duck_diagnostic::{
  Diagnostic, DiagnosticCode, DiagnosticEngine, DiagnosticFormatter, RenderOptions, Severity,
};

/// Print every diagnostic in `engine` to stderr, picking with-source vs
/// compact rendering per diagnostic. Color follows TTY detection on stderr;
/// override with `color = Some(true|false)`.
pub fn print_all_smart<C: DiagnosticCode>(engine: &DiagnosticEngine<C>, color: Option<bool>) {
  let color = color.unwrap_or_else(|| std::io::stderr().is_terminal());
  eprint!("{}", format_all_smart(engine, color));
}

/// Format every diagnostic in `engine` to a single string, picking
/// with-source vs compact rendering per diagnostic. Includes the trailing
/// summary line.
pub fn format_all_smart<C: DiagnosticCode>(engine: &DiagnosticEngine<C>, color: bool) -> String {
  let mut sources: HashMap<Arc<str>, Option<String>> = HashMap::new();
  let mut out = String::new();

  for d in engine.iter() {
    out.push_str(&format_one_smart(d, &mut sources, color));
  }

  out.push_str(&summary_line(engine, color));
  out
}

fn format_one_smart<C: DiagnosticCode>(
  d: &Diagnostic<C>,
  cache: &mut HashMap<Arc<str>, Option<String>>,
  color: bool,
) -> String {
  let Some(label) = d.primary_label() else {
    return d.format_compact(color);
  };
  if label.span.line == 0 {
    return d.format_compact(color);
  }
  let file = label.span.file.clone();
  let entry = cache.entry(file.clone()).or_insert_with(|| std::fs::read_to_string(&*file).ok());
  match entry {
    Some(src) => {
      let opts = RenderOptions { color, ..Default::default() };
      DiagnosticFormatter::new(d, src).with_options(opts).format()
    },
    None => d.format_compact(color),
  }
}

fn summary_line<C: DiagnosticCode>(engine: &DiagnosticEngine<C>, color: bool) -> String {
  let errors = engine.iter().filter(|d| matches!(d.severity, Severity::Error | Severity::Bug)).count();
  let warnings = engine.iter().filter(|d| d.severity == Severity::Warning).count();
  if errors == 0 && warnings == 0 {
    return String::new();
  }
  // Re-use upstream summary by piping through format_all_compact_plain on
  // an empty-bodied engine isn't possible — so we render a compact summary
  // ourselves to keep wording aligned.
  if errors > 0 {
    let warn_part = if warnings > 0 {
      format!("; {} warning{} emitted", warnings, if warnings == 1 { "" } else { "s" })
    } else {
      String::new()
    };
    if color {
      format!(
        "\x1b[1;31merror\x1b[0m: could not compile due to {} previous error{}{}\n",
        errors,
        if errors == 1 { "" } else { "s" },
        warn_part,
      )
    } else {
      format!(
        "error: could not compile due to {} previous error{}{}\n",
        errors,
        if errors == 1 { "" } else { "s" },
        warn_part,
      )
    }
  } else if color {
    format!(
      "\x1b[1;33mwarning\x1b[0m: {} warning{} emitted\n",
      warnings,
      if warnings == 1 { "" } else { "s" },
    )
  } else {
    format!("warning: {} warning{} emitted\n", warnings, if warnings == 1 { "" } else { "s" })
  }
}
