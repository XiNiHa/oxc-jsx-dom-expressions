use html_escape::decode_html_entities;
use oxc::{
    allocator::Vec as OxcVec,
    ast::ast::{self},
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
        let tag_name = match &el.opening_element.name {
            ast::JSXElementName::Identifier(ident) => ident.name.clone(),
            _ => {
                // TODO
                return TransformResult {
                    id: None,
                    template: None,
                    exprs: ctx.ast.vec(),
                    text: false,
                };
            }
        };

        let attributes = self.generate_attributes(&el.opening_element.attributes);
        let child_templates = self.generate_child_templates(&el.children, ctx);

        // TODO
        let template = format!("<{}{}>{}", tag_name, attributes, child_templates);

        TransformResult {
            id: None,
            template: Some(template),
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

    /// generate attributes string without quotes around values
    fn generate_attributes(&self, attrs: &[ast::JSXAttributeItem<'a>]) -> String {
        let mut attrs_string = String::new();
        for attr_item in attrs {
            let ast::JSXAttributeItem::Attribute(attr) = attr_item else {
                continue;
            };
            let ast::JSXAttributeName::Identifier(ident) = &attr.name else {
                continue;
            };
            let name = ident.name.as_ref();

            match &attr.value {
                Some(ast::JSXAttributeValue::StringLiteral(str_lit)) => {
                    let value = str_lit.value.as_ref();
                    attrs_string.push_str(&format!(" {}={}", name, value));
                }
                Some(_) => {
                    // TODO
                }
                None => {
                    // attributes without a value (e.g., <input disabled />)
                    attrs_string.push_str(&format!(" {}", name));
                }
            }
        }
        attrs_string
    }

    /// Process children and collect their templates
    fn generate_child_templates(
        &self,
        children: &[ast::JSXChild<'a>],
        ctx: &mut TraverseCtx<'a>,
    ) -> String {
        let info = TransformInfo::default();
        children
            .iter()
            .filter_map(|child| {
                self.transform_node(child, ctx, &info)
                    .and_then(|child_result| child_result.template)
            })
            .collect::<String>()
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
                // if template is not null, create template => import { template as _$template } from "solid-js/web"; ... (client side rendering)
                ctx.ast.expression_null_literal(SPAN)
            }
        }
    }
}

#[cfg(test)]
mod transform_tests {
    use super::*;
    use oxc::{allocator::Allocator, parser::Parser, semantic::SemanticBuilder, span::SourceType};

    struct TestCase {
        source: &'static str,
        expected_id: Option<Atom<'static>>,
        expected_template: Option<String>,
        expected_exprs_len: usize,
        expected_text: bool,
    }

    #[test]
    fn test_transform_element() {
        let test_cases = vec![
            /* solidJS client side rendering result
                import { template as _$template } from "solid-js/web";
                var _tmpl$ = /*#__PURE__*/_$template(`<div class=test-class>Hello`); // <-
                const foo = _tmpl$();
            */
            TestCase {
                source: r#"<div class="test-class">Hello</div>"#,
                expected_id: None,
                expected_template: Some(r#"<div class=test-class>Hello"#.to_string()),
                expected_exprs_len: 0,
                expected_text: false,
            },
            /* solidJS client side rendering result
                import { template as _$template } from "solid-js/web";
                var _tmpl$ = /*#__PURE__*/_$template(`<div>Hello`); // <-
                const foo = _tmpl$();
            */
            TestCase {
                source: r#"<div>Hello</div>"#,
                expected_id: None,
                expected_template: Some(r#"<div>Hello"#.to_string()),
                expected_exprs_len: 0,
                expected_text: false,
            },
            /* solidJS client side rendering result
                import { template as _$template } from "solid-js/web";
                var _tmpl$ = /*#__PURE__*/_$template(`<span class=highlight>Text`); // <-
                const foo = _tmpl$();
            */
            TestCase {
                source: r#"<span class="highlight">Text</span>"#,
                expected_id: None,
                expected_template: Some(r#"<span class=highlight>Text"#.to_string()),
                expected_exprs_len: 0,
                expected_text: false,
            },
        ];

        for case in test_cases {
            let allocator = Allocator::default();
            let source_type = SourceType::jsx();

            let parse_result = Parser::new(&allocator, case.source, source_type).parse();
            let program = parse_result.program;

            let semantic_result = SemanticBuilder::new(case.source)
                .with_excess_capacity(2.0)
                .build(&program);
            let (symbols, scopes) = semantic_result.semantic.into_symbol_table_and_scope_tree();

            if let ast::Statement::ExpressionStatement(expr_stmt) = &program.body[0] {
                if let ast::Expression::JSXElement(jsx_element) = &expr_stmt.expression {
                    let mut ctx = TraverseCtx::new(scopes, symbols, &allocator);
                    let config = Config {
                        generate: OutputType::Dom,
                        ..Default::default()
                    };
                    let transform = JsxTransform::new(config);

                    let result = transform.transform_element(jsx_element, &mut ctx);

                    assert_eq!(
                        result.id, case.expected_id,
                        "Failed for source: {}",
                        case.source
                    );
                    assert_eq!(
                        result.template, case.expected_template,
                        "Failed for source: {}",
                        case.source
                    );
                    assert_eq!(
                        result.exprs.len(),
                        case.expected_exprs_len,
                        "Failed for source: {}",
                        case.source
                    );
                    assert_eq!(
                        result.text, case.expected_text,
                        "Failed for source: {}",
                        case.source
                    );
                } else {
                    panic!("Expected JSXElement for source: {}", case.source);
                }
            } else {
                panic!("Expected ExpressionStatement for source: {}", case.source);
            }
        }
    }
}
