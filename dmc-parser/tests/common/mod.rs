#![allow(dead_code)]
use dmc_parser::ast::*;
use dmc_parser::parse;

pub fn parse_doc(src: &str) -> Document {
  parse(src)
}
