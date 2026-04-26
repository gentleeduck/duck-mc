#![allow(dead_code)]
use duck_md_parser::ast::*;
use duck_md_parser::parse;

pub fn parse_doc(src: &str) -> Document {
  parse(src)
}
