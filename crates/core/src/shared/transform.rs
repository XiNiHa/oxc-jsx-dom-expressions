use html_escape::decode_html_entities;
use oxc::{
    allocator::Vec as OxcVec,
    ast::ast,
    semantic::SymbolFlags,
    span::{Atom, SPAN},
};
use oxc_traverse::{Traverse, TraverseCtx};

use crate::{shared::utils::jsx_text_to_str, Config, OutputType};

pub struct JsxTransform {
    config: Config,
}

impl JsxTransform {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[derive(Default)]
struct TransformInfo {
    top_level: bool,
    skip_id: bool,
    last_element: bool,
    do_not_escape: bool,
}

struct TransformResult<'a> {
    id: Option<Atom<'a>>,
    template: Option<String>,
    exprs: OxcVec<'a, ast::Expression<'a>>,
    text: bool,
}

impl<'a> Traverse<'a> for JsxTransform {
    fn enter_expression(
        &mut self,
        node: &mut ast::Expression<'a>,
        ctx: &mut oxc_traverse::TraverseCtx<'a>,
    ) {
        match node {
            ast::Expression::JSXElement(_) => {
                let ast::Expression::JSXElement(el) = ctx.ast.move_expression(node) else {
                    return;
                };
                let result = self.transform_node(
                    &ctx.ast.jsx_child_from_jsx_element(el),
                    ctx,
                    &Default::default(),
                );
                *node = result
                    .map(|r| r.create_template(&self.config, ctx))
                    .unwrap_or_else(|| ctx.ast.expression_null_literal(SPAN));
            }
            ast::Expression::JSXFragment(_) => {
                let ast::Expression::JSXFragment(frag) = ctx.ast.move_expression(node) else {
                    return;
                };
                let result = self.transform_node(
                    &ctx.ast.jsx_child_from_jsx_fragment(frag),
                    ctx,
                    &TransformInfo {
                        top_level: true,
                        last_element: true,
                        ..Default::default()
                    },
                );
                *node = result
                    .map(|r| r.create_template(&self.config, ctx))
                    .unwrap_or_else(|| ctx.ast.expression_null_literal(SPAN));
            }
            _ => {}
        }
    }
}

impl<'a> JsxTransform {
    fn transform_node(
        &self,
        node: &ast::JSXChild<'a>,
        ctx: &mut TraverseCtx<'a>,
        info: &TransformInfo,
    ) -> Option<TransformResult<'a>> {
        match node {
            ast::JSXChild::Element(el) => Some(self.transform_element(el, ctx)),
            ast::JSXChild::Fragment(frag) => {
                Some(self.transform_fragment_children(&frag.children, ctx, info))
            }
            ast::JSXChild::Text(text) => match jsx_text_to_str(&text.value) {
                str if str.is_empty() => None,
                str => Some(TransformResult {
                    id: match info.skip_id {
                        true => None,
                        false => Some(
                            ctx.generate_uid_in_current_scope(
                                "el$",
                                SymbolFlags::FunctionScopedVariable,
                            )
                            .name,
                        ),
                    },
                    template: Some(str),
                    text: true,
                    exprs: ctx.ast.vec(),
                }),
            },
            ast::JSXChild::ExpressionContainer(container) => {
                // TODO
                Some(TransformResult {
                    id: None,
                    template: None,
                    exprs: ctx.ast.vec(),
                    text: false,
                })
            }
            ast::JSXChild::Spread(spread) => {
                // TODO
                Some(TransformResult {
                    id: None,
                    template: None,
                    exprs: ctx.ast.vec(),
                    text: false,
                })
            }
        }
    }

    fn transform_element(
        &self,
        el: &ast::JSXElement<'a>,
        ctx: &mut TraverseCtx<'a>,
    ) -> TransformResult<'a> {
        // TODO
        TransformResult {
            id: None,
            template: None,
            exprs: ctx.ast.vec(),
            text: false,
        }
    }

    fn transform_fragment_children(
        &self,
        children: &OxcVec<'a, ast::JSXChild<'a>>,
        ctx: &mut TraverseCtx<'a>,
        info: &TransformInfo,
    ) -> TransformResult<'a> {
        let filtered = children.iter().filter(|child| match child {
            ast::JSXChild::ExpressionContainer(container) => {
                !matches!(container.expression, ast::JSXExpression::EmptyExpression(_))
            }
            // TODO: this doesn't 100% match with the original behavior
            // (https://github.com/ryansolid/dom-expressions/blob/388985beae617521fe7daff06759e9d704b852fa/packages/babel-plugin-jsx-dom-expressions/src/shared/utils.js#L196)
            ast::JSXChild::Text(text) => !text.value.trim().is_empty(),
            _ => true,
        });
        let child_nodes = ctx
            .ast
            .vec_from_iter(filtered.filter_map(|child| match child {
                ast::JSXChild::Text(text) => {
                    let v = jsx_text_to_str(&text.value);
                    let v = decode_html_entities(&v);
                    match v.is_empty() {
                        true => None,
                        false => Some(ctx.ast.expression_string_literal(text.span, v)),
                    }
                }
                child => {
                    let child_result = self.transform_node(child, ctx, info);
                    child_result
                        .as_ref()
                        .map(|r| r.create_template(&self.config, ctx))
                }
            }));
        TransformResult {
            exprs: match child_nodes.len() > 1 {
                true => ctx.ast.vec1(
                    ctx.ast.expression_array(
                        SPAN,
                        ctx.ast.vec_from_iter(
                            child_nodes
                                .into_iter()
                                .map(|expr| ctx.ast.array_expression_element_expression(expr)),
                        ),
                        None,
                    ),
                ),
                false => child_nodes,
            },
            id: None,
            template: None,
            text: false,
        }
    }
}

impl<'a> TransformResult<'a> {
    fn create_template(
        &self,
        config: &Config,
        ctx: &mut oxc_traverse::TraverseCtx<'a>,
    ) -> ast::Expression<'a> {
        match config.generate {
            OutputType::Dom => {
                // TODO
                ctx.ast.expression_null_literal(SPAN)
            }
        }
    }
}
