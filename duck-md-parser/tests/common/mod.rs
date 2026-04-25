#![allow(dead_code)]
use duck_md_parser::parse;
use duck_md_parser::ast::*;

pub fn parse_doc(src: &str) -> Document {
    parse(src)
}
