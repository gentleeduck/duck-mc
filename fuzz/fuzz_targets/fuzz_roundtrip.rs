#![no_main]
//! Fuzz target: parse -> render HTML. Exercises the codegen emitter on
//! whatever AST the parser produces. Must terminate without panic.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
  let s = String::from_utf8_lossy(data);
  let doc = dmc_parser::parse(&s);
  let _ = dmc_codegen::render_html(&doc);
});
