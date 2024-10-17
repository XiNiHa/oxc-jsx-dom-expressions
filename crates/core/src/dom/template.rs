use oxc::{ast::ast, span::SPAN};
use oxc_traverse::TraverseCtx;

use crate::{
    shared::transform::{TemplateCreationCtx, TransformResult},
    Config,
};

impl<'a> TransformResult<'a> {
    pub fn create_template_dom(
        &self,
        config: &Config,
        traverse_ctx: &mut TraverseCtx<'a>,
        creation_ctx: &mut TemplateCreationCtx<'a>,
        wrap: bool,
    ) -> ast::Expression<'a> {
        // TODO
        traverse_ctx.ast.expression_null_literal(SPAN)
    }
}
