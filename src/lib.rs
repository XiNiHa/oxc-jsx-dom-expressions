use napi_derive::*;
use oxc::{allocator::Allocator, codegen::Codegen, parser::Parser, span::SourceType};

#[napi]
pub fn roundtrip(source: String) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let parse_result = Parser::new(&allocator, &source, source_type).parse();
    let codegen_result = Codegen::new().build(&parse_result.program);
    codegen_result.code
}
