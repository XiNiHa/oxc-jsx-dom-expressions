use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CodegenOptions},
    parser::Parser,
    span::SourceType,
};

mod dom;

fn roundtrip(source: &str) -> String {
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
    let program = allocator.alloc(ret.program);
    CodeGenerator::new()
        .with_options(CodegenOptions {
            single_quote: true,
            ..CodegenOptions::default()
        })
        .build(program)
        .code
}
