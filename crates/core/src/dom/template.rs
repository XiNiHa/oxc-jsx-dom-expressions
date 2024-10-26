use oxc::{
    ast::{ast, NONE},
    semantic::{ReferenceFlags, SymbolFlags},
    span::{Atom, SPAN},
};
use oxc_traverse::TraverseCtx;

use crate::{
    shared::{
        transform::{Template, TemplateCreationCtx, TransformResult},
        utils::register_import_method,
    },
    Config, OutputType,
};

impl<'a> TransformResult<'a> {
    pub fn create_template_dom(
        mut self,
        config: &Config,
        traverse_ctx: &mut TraverseCtx<'a>,
        creation_ctx: &mut TemplateCreationCtx<'a>,
        wrap: bool,
    ) -> ast::Expression<'a> {
        if let Some(id) = &self.id {
            self.register_template_dom(id.clone(), config, traverse_ctx, creation_ctx);
            match self.exprs.is_empty() && self.declarators.len() == 1 {
                true => self.declarators.into_iter().next().unwrap().1,
                false => traverse_ctx.ast.expression_call(
                    SPAN,
                    traverse_ctx.ast.expression_arrow_function(
                        SPAN,
                        false,
                        false,
                        NONE,
                        traverse_ctx.ast.formal_parameters(
                            SPAN,
                            ast::FormalParameterKind::ArrowFormalParameters,
                            traverse_ctx.ast.vec(),
                            NONE,
                        ),
                        NONE,
                        traverse_ctx.ast.function_body(
                            SPAN,
                            traverse_ctx.ast.vec(),
                            traverse_ctx.ast.vec_from_iter([traverse_ctx
                                .ast
                                .statement_declaration(traverse_ctx.ast.declaration_variable(
                                    SPAN,
                                    ast::VariableDeclarationKind::Var,
                                    traverse_ctx.ast.vec_from_iter(
                                        self.declarators.into_iter().map(|(id, decl)| {
                                            traverse_ctx.ast.variable_declarator(
                                                SPAN,
                                                ast::VariableDeclarationKind::Var,
                                                traverse_ctx.ast.binding_pattern(
                                                    traverse_ctx
                                                        .ast
                                                        .binding_pattern_kind_binding_identifier(
                                                            SPAN, id,
                                                        ),
                                                    NONE,
                                                    false,
                                                ),
                                                Some(decl),
                                                false,
                                            )
                                        }),
                                    ),
                                    false,
                                ))]),
                        ),
                    ),
                    NONE,
                    traverse_ctx.ast.vec(),
                    false,
                ),
            }
        } else if wrap && self.dynamic && !config.memo_wrapper.is_empty() {
            traverse_ctx.ast.expression_call(
                SPAN,
                register_import_method(
                    &mut creation_ctx.imports,
                    &config.memo_wrapper,
                    &config.module_name,
                    traverse_ctx,
                )
                .create_read_expression(traverse_ctx),
                NONE,
                traverse_ctx.ast.vec_from_iter(
                    self.exprs
                        .into_iter()
                        .take(1)
                        .map(|e| traverse_ctx.ast.argument_expression(e)),
                ),
                false,
            )
        } else {
            self.exprs
                .into_iter()
                .next()
                .unwrap_or_else(|| traverse_ctx.ast.expression_null_literal(SPAN))
        }
    }

    fn register_template_dom(
        &mut self,
        id: Atom<'a>,
        config: &Config,
        traverse_ctx: &mut TraverseCtx<'a>,
        creation_ctx: &mut TemplateCreationCtx<'a>,
    ) {
        let Some(template) = &self.template else {
            return;
        };

        let template_id = match &self.skip_template {
            true => None,
            false => Some(
                creation_ctx
                    .templates
                    .iter()
                    .find(|t| &t.template == template)
                    .map(|t| t.id.clone())
                    .unwrap_or_else(|| {
                        let id = traverse_ctx
                            .generate_uid_in_current_scope(
                                "tmpl$",
                                SymbolFlags::FunctionScopedVariable,
                            )
                            .name;
                        creation_ctx.templates.push(Template {
                            id: id.clone(),
                            template: template.clone(),
                            renderer: OutputType::Dom,
                        });
                        id
                    }),
            ),
        };

        let decl_init = match config.hydratable {
            true => traverse_ctx.ast.expression_call(
                SPAN,
                register_import_method(
                    &mut creation_ctx.imports,
                    "getNextElement",
                    &config.module_name,
                    traverse_ctx,
                )
                .create_expression(ReferenceFlags::Read, traverse_ctx),
                NONE,
                match template_id {
                    Some(id) => traverse_ctx.ast.vec1(traverse_ctx.ast.argument_expression(
                        traverse_ctx.ast.expression_identifier_reference(SPAN, id),
                    )),
                    None => traverse_ctx.ast.vec(),
                },
                false,
            ),
            false => traverse_ctx.ast.expression_call(
                SPAN,
                traverse_ctx.ast.expression_identifier_reference(
                    SPAN,
                    template_id.expect("template_id not expected to be None if !config.hydratable"),
                ),
                NONE,
                traverse_ctx.ast.vec(),
                false,
            ),
        };

        self.declarators.push((id.clone(), decl_init));
    }
}
