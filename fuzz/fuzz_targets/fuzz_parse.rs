#![no_main]
//! Fuzz target: `dmc_parser::parse` must terminate without panic on any
//! input. Non-UTF-8 bytes are coerced via `from_utf8_lossy` so the parser
//! itself (not the UTF-8 gate) is exercised.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
  let s = String::from_utf8_lossy(data);
  let _ = dmc_parser::parse(&s);
});
