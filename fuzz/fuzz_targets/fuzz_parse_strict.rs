#![no_main]
//! Fuzz target: `dmc_parser::parse_with` under the strict spec-runner
//! options (CM raw-HTML blocks + GFM autolinks + legacy GFM emphasis).
//! Must terminate without panic on any input.

use dmc_parser::ParseOptions;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
  let s = String::from_utf8_lossy(data);
  let _ = dmc_parser::parse_with(
    &s,
    ParseOptions { cm_strict_html_blocks: true, gfm_autolinks: true, legacy_gfm_emphasis: true },
  );
});
