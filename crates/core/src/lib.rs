use std::convert::Infallible;

use oxc::{
    allocator::Allocator, codegen::Codegen, parser::Parser, semantic::SemanticBuilder,
    span::SourceType,
};

pub mod config;
mod shared;

pub use config::*;
use shared::{this_to_self::ThisToSelfTransform, transform::JsxTransform};

pub fn transform(source: String, config: Config) -> Result<String, Infallible> {
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();

    let parse_result = Parser::new(&allocator, &source, source_type).parse();
    let mut program = parse_result.program;
    let semantic_result = SemanticBuilder::new(&source)
        .with_excess_capacity(2.0)
        .build(&program);
    let (symbols, scopes) = semantic_result.semantic.into_symbol_table_and_scope_tree();

    let mut this_transform = ThisToSelfTransform::new();
    let (symbols, scopes) = oxc_traverse::traverse_mut(
        &mut this_transform,
        &allocator,
        &mut program,
        symbols,
        scopes,
    );

    let mut jsx_transform = JsxTransform::new(config);
    oxc_traverse::traverse_mut(
        &mut jsx_transform,
        &allocator,
        &mut program,
        symbols,
        scopes,
    );

    Ok(Codegen::new().build(&program).code)
}
