use std::collections::{hash_map::Entry, HashMap};

use html_escape::decode_html_entities;
use oxc::{
    allocator::{CloneIn, IntoIn, Vec as OxcVec},
    ast::{
        ast::{self},
        NONE,
    },
    semantic::{ScopeFlags, SymbolFlags},
    span::{Atom, SPAN},
};
use oxc_traverse::{BoundIdentifier, Traverse, TraverseCtx};

use crate::{shared::utils::jsx_text_to_str, Config, OutputType};

use super::utils::DynamicChecker;

pub struct JsxTransform<'a> {
    config: Config,
    template_creation_ctx: TemplateCreationCtx<'a>,
}

impl<'a> JsxTransform<'a> {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            template_creation_ctx: TemplateCreationCtx {
                templates: Vec::new(),
                imports: HashMap::new(),
            },
        }
    }
}

#[derive(Default)]
pub struct TransformInfo {
    pub top_level: bool,
    pub skip_id: bool,
    pub last_element: bool,
    pub do_not_escape: bool,
    pub component_child: bool,
    pub fragment_child: bool,
}

pub struct TransformResult<'a> {
    pub id: Option<Atom<'a>>,
    pub template: Option<String>,
    pub exprs: OxcVec<'a, ast::Expression<'a>>,
    pub declarators: OxcVec<'a, (Atom<'a>, ast::Expression<'a>)>,
    pub text: bool,
    pub dynamic: bool,
    pub skip_template: bool,
}

impl<'a> Traverse<'a> for JsxTransform<'a> {
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
                    .map(|r| {
                        r.create_template(&self.config, ctx, &mut self.template_creation_ctx, false)
                    })
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
                    .map(|r| {
                        r.create_template(&self.config, ctx, &mut self.template_creation_ctx, false)
                    })
                    .unwrap_or_else(|| ctx.ast.expression_null_literal(SPAN));
            }
            _ => {}
        }
    }

    fn exit_program(&mut self, node: &mut ast::Program<'a>, ctx: &mut TraverseCtx<'a>) {
        node.body.splice(
            0..0,
            self.template_creation_ctx
                .get_leading_stmts(&self.config.module_name, ctx),
        );
    }
}

impl<'a> JsxTransform<'a> {
    pub fn transform_node(
        &mut self,
        node: &ast::JSXChild<'a>,
        ctx: &mut TraverseCtx<'a>,
        info: &TransformInfo,
    ) -> Option<TransformResult<'a>> {
        match node {
            ast::JSXChild::Element(el) => Some(self.transform_element(el, ctx, info)),
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
                    exprs: ctx.ast.vec(),
                    declarators: ctx.ast.vec(),
                    text: true,
                    dynamic: false,
                    skip_template: false,
                }),
            },
            ast::JSXChild::ExpressionContainer(container) => {
                if matches!(container.expression, ast::JSXExpression::EmptyExpression(_)) {
                    return None;
                }
                let is_dynamic = DynamicChecker::new()
                    .check_member(true)
                    .check_tags(info.component_child)
                    .native(!info.component_child);
                if !is_dynamic.check(&container.expression) {
                    return Some(TransformResult {
                        id: None,
                        template: None,
                        exprs: ctx.ast.vec1(
                            container
                                .expression
                                .to_expression()
                                .clone_in(ctx.ast.allocator),
                        ),
                        text: false,
                        skip_template: false,
                        declarators: ctx.ast.vec(),
                        dynamic: false,
                    });
                }
                let (statement, result) = match &container.expression {
                    ast::JSXExpression::LogicalExpression(logical_expression) => self
                        .transform_logical_expression(
                            logical_expression,
                            ctx,
                            info.component_child || info.fragment_child,
                            false,
                        ),
                    ast::JSXExpression::ConditionalExpression(conditional_expression) => self
                        .transform_conditional_expression(
                            conditional_expression,
                            ctx,
                            info.component_child || info.fragment_child,
                            false,
                        ),
                    ast::JSXExpression::CallExpression(call_expression)
                        if !info.component_child
                            && !call_expression.callee.is_call_expression()
                            && !call_expression.callee.is_member_expression()
                            && call_expression.arguments.is_empty() =>
                    {
                        (None, call_expression.callee.clone_in(ctx.ast.allocator))
                    }
                    _ => (
                        None,
                        ast::Expression::ArrowFunctionExpression(
                            ctx.ast.alloc_arrow_function_expression_with_scope_id(
                                SPAN,
                                true,
                                false,
                                NONE,
                                ctx.ast.formal_parameters(
                                    SPAN,
                                    ast::FormalParameterKind::ArrowFormalParameters,
                                    ctx.ast.vec(),
                                    NONE,
                                ),
                                NONE,
                                ctx.ast.function_body(
                                    SPAN,
                                    ctx.ast.vec(),
                                    ctx.ast.vec1(ast::Statement::ExpressionStatement(
                                        ctx.ast.alloc_expression_statement(
                                            SPAN,
                                            container
                                                .expression
                                                .to_expression()
                                                .clone_in(ctx.ast.allocator),
                                        ),
                                    )),
                                ),
                                ctx.create_child_scope_of_current(ScopeFlags::Arrow),
                            ),
                        ),
                    ),
                };
                let expr = if let Some(statement) = statement {
                    ctx.ast.expression_call(
                        SPAN,
                        ast::Expression::ArrowFunctionExpression(
                            ctx.ast.alloc_arrow_function_expression_with_scope_id(
                                SPAN,
                                false,
                                false,
                                NONE,
                                ctx.ast.formal_parameters(
                                    SPAN,
                                    ast::FormalParameterKind::ArrowFormalParameters,
                                    ctx.ast.vec(),
                                    NONE,
                                ),
                                NONE,
                                ctx.ast.function_body(
                                    SPAN,
                                    ctx.ast.vec(),
                                    ctx.ast.vec_from_iter([
                                        statement,
                                        ctx.ast.statement_return(SPAN, Some(result)),
                                    ]),
                                ),
                                ctx.create_child_scope_of_current(ScopeFlags::Arrow),
                            ),
                        ),
                        NONE,
                        ctx.ast.vec(),
                        false,
                    )
                } else {
                    result
                };
                Some(TransformResult {
                    id: None,
                    template: None,
                    exprs: ctx.ast.vec1(expr),
                    declarators: ctx.ast.vec(),
                    text: false,
                    dynamic: true,
                    skip_template: false,
                })
            }
            ast::JSXChild::Spread(spread) => {
                // TODO
                Some(TransformResult {
                    id: None,
                    template: None,
                    exprs: ctx.ast.vec(),
                    declarators: ctx.ast.vec(),
                    text: false,
                    dynamic: false,
                    skip_template: false,
                })
            }
        }
    }

    pub fn transform_element(
        &mut self,
        el: &ast::JSXElement<'a>,
        ctx: &mut TraverseCtx<'a>,
        info: &TransformInfo,
    ) -> TransformResult<'a> {
        match self.config.generate {
            OutputType::Dom => self.transform_element_dom(el, ctx, info),
        }
    }

    pub fn transform_fragment_children(
        &mut self,
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
                    child_result.map(|r| {
                        r.create_template(&self.config, ctx, &mut self.template_creation_ctx, true)
                    })
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
            declarators: ctx.ast.vec(),
            text: false,
            dynamic: false,
            skip_template: false,
        }
    }

    fn transform_short_circuit(
        &mut self,
        memo: BoundIdentifier<'a>,
        transformed: ast::Expression<'a>,
        short_circuit: Option<(BoundIdentifier<'a>, ast::Expression<'a>)>,
        ctx: &mut TraverseCtx<'a>,
        deep: bool,
    ) -> (Option<ast::Statement<'a>>, ast::Expression<'a>) {
        if let Some((identifier, condition)) = short_circuit {
            let callee = ast::Expression::ArrowFunctionExpression(ctx.ast.alloc(
                ctx.ast.arrow_function_expression_with_scope_id(
                    SPAN,
                    true,
                    false,
                    NONE,
                    ctx.ast.formal_parameters(
                        SPAN,
                        ast::FormalParameterKind::ArrowFormalParameters,
                        ctx.ast.vec(),
                        NONE,
                    ),
                    NONE,
                    ctx.ast.function_body(
                        SPAN,
                        ctx.ast.vec(),
                        ctx.ast.vec1(ctx.ast.statement_expression(SPAN, condition)),
                    ),
                    ctx.create_child_scope_of_current(ScopeFlags::Arrow),
                ),
            ));
            let statement = ctx.ast.statement_declaration(ctx.ast.declaration_variable(
                SPAN,
                ast::VariableDeclarationKind::Var,
                ctx.ast.vec1(ctx.ast.variable_declarator(
                    SPAN,
                    ast::VariableDeclarationKind::Var,
                    identifier.create_binding_pattern(ctx),
                    Some(if self.config.memo_wrapper.is_empty() {
                        callee
                    } else {
                        ctx.ast.expression_call(
                            SPAN,
                            memo.create_read_expression(ctx),
                            NONE,
                            ctx.ast.vec1(ctx.ast.argument_expression(callee)),
                            false,
                        )
                    }),
                    false,
                )),
                false,
            ));
            let result = ast::Expression::ArrowFunctionExpression(
                ctx.ast.alloc(
                    ctx.ast.arrow_function_expression_with_scope_id(
                        SPAN,
                        true,
                        false,
                        NONE,
                        ctx.ast.formal_parameters(
                            SPAN,
                            ast::FormalParameterKind::ArrowFormalParameters,
                            ctx.ast.vec(),
                            NONE,
                        ),
                        NONE,
                        ctx.ast.function_body(
                            SPAN,
                            ctx.ast.vec(),
                            ctx.ast
                                .vec1(ctx.ast.statement_expression(SPAN, transformed)),
                        ),
                        ctx.create_child_scope_of_current(ScopeFlags::Arrow),
                    ),
                ),
            );
            if deep {
                let scope_id = ctx.create_child_scope_of_current(ScopeFlags::Arrow);
                (
                    None,
                    ctx.ast.expression_call(
                        SPAN,
                        ast::Expression::ArrowFunctionExpression(
                            ctx.ast.alloc_arrow_function_expression_with_scope_id(
                                SPAN,
                                false,
                                false,
                                NONE,
                                ctx.ast.formal_parameters(
                                    SPAN,
                                    ast::FormalParameterKind::ArrowFormalParameters,
                                    ctx.ast.vec(),
                                    NONE,
                                ),
                                NONE,
                                ctx.ast.function_body(
                                    SPAN,
                                    ctx.ast.vec(),
                                    ctx.ast.vec_from_iter([
                                        statement,
                                        ctx.ast.statement_expression(SPAN, result),
                                    ]),
                                ),
                                scope_id,
                            ),
                        ),
                        NONE,
                        ctx.ast.vec(),
                        false,
                    ),
                )
            } else {
                (Some(statement), result)
            }
        } else {
            (
                None,
                if deep {
                    transformed
                } else {
                    ast::Expression::ArrowFunctionExpression(
                        ctx.ast.alloc_arrow_function_expression_with_scope_id(
                            SPAN,
                            true,
                            false,
                            NONE,
                            ctx.ast.formal_parameters(
                                SPAN,
                                ast::FormalParameterKind::ArrowFormalParameters,
                                ctx.ast.vec(),
                                NONE,
                            ),
                            NONE,
                            ctx.ast.function_body(
                                SPAN,
                                ctx.ast.vec(),
                                ctx.ast
                                    .vec1(ctx.ast.statement_expression(SPAN, transformed)),
                            ),
                            ctx.create_child_scope_of_current(ScopeFlags::Arrow),
                        ),
                    )
                },
            )
        }
    }

    pub fn transform_logical_expression(
        &mut self,
        logical_expression: &ast::LogicalExpression<'a>,
        ctx: &mut TraverseCtx<'a>,
        inline: bool,
        deep: bool,
    ) -> (Option<ast::Statement<'a>>, ast::Expression<'a>) {
        let memo = self.template_creation_ctx.register_import_method(
            &self.config.memo_wrapper,
            &self.config.module_name,
            ctx,
        );
        let (transformed, short_circuit) =
            if matches!(logical_expression.operator, ast::LogicalOperator::And)
                && DynamicChecker::new()
                    .check_call_expressions(true)
                    .check_tags(true)
                    .check(&logical_expression.right)
                && DynamicChecker::new()
                    .check_call_expressions(true)
                    .check_member(true)
                    .check(&logical_expression.left)
            {
                let left = logical_expression.left.clone_in(&ctx.ast.allocator);
                let condition = if matches!(left, ast::Expression::BinaryExpression(_)) {
                    left
                } else {
                    ctx.ast.expression_unary(
                        SPAN,
                        ast::UnaryOperator::LogicalNot,
                        ctx.ast
                            .expression_unary(SPAN, ast::UnaryOperator::LogicalNot, left),
                    )
                };
                let (callee, short_circuit) = if inline {
                    (
                        ctx.ast.expression_call(
                            SPAN,
                            memo.create_read_expression(ctx),
                            NONE,
                            ctx.ast.vec1(ctx.ast.argument_expression(
                                ast::Expression::ArrowFunctionExpression(ctx.ast.alloc(
                                    ctx.ast.arrow_function_expression_with_scope_id(
                                        SPAN,
                                        true,
                                        false,
                                        NONE,
                                        ctx.ast.formal_parameters(
                                            SPAN,
                                            ast::FormalParameterKind::ArrowFormalParameters,
                                            ctx.ast.vec(),
                                            NONE,
                                        ),
                                        NONE,
                                        ctx.ast.function_body(
                                            SPAN,
                                            ctx.ast.vec(),
                                            ctx.ast.vec1(
                                                ctx.ast.statement_expression(SPAN, condition),
                                            ),
                                        ),
                                        ctx.create_child_scope_of_current(ScopeFlags::Arrow),
                                    ),
                                )),
                            )),
                            false,
                        ),
                        None,
                    )
                } else {
                    let identifier = ctx
                        .generate_uid_in_current_scope("_c$", SymbolFlags::FunctionScopedVariable);
                    (
                        identifier.create_read_expression(ctx),
                        Some((identifier, condition)),
                    )
                };
                (
                    ctx.ast.expression_logical(
                        SPAN,
                        ctx.ast
                            .expression_call(SPAN, callee, NONE, ctx.ast.vec(), false),
                        logical_expression.operator,
                        logical_expression.right.clone_in(ctx.ast.allocator),
                    ),
                    short_circuit,
                )
            } else {
                (
                    ctx.ast
                        .expression_from_logical(logical_expression.clone_in(ctx.ast.allocator)),
                    None,
                )
            };
        self.transform_short_circuit(memo, transformed, short_circuit, ctx, deep)
    }

    pub fn transform_conditional_expression(
        &mut self,
        conditional_expression: &ast::ConditionalExpression<'a>,
        ctx: &mut TraverseCtx<'a>,
        inline: bool,
        deep: bool,
    ) -> (Option<ast::Statement<'a>>, ast::Expression<'a>) {
        let memo = self.template_creation_ctx.register_import_method(
            &self.config.memo_wrapper,
            &self.config.module_name,
            ctx,
        );
        let tags_checker = DynamicChecker::new()
            .check_call_expressions(true)
            .check_tags(true);
        let (transformed, short_circuit) = if (tags_checker
            .check(&conditional_expression.consequent)
            || tags_checker.check(&conditional_expression.alternate))
            && DynamicChecker::new()
                .check_call_expressions(true)
                .check_member(true)
                .check(&conditional_expression.test)
        {
            let test = conditional_expression.test.clone_in(&ctx.ast.allocator);
            let condition = if matches!(test, ast::Expression::BinaryExpression(_)) {
                test
            } else {
                ctx.ast.expression_unary(
                    SPAN,
                    ast::UnaryOperator::LogicalNot,
                    ctx.ast
                        .expression_unary(SPAN, ast::UnaryOperator::LogicalNot, test),
                )
            };
            let consequent = conditional_expression
                .consequent
                .clone_in(ctx.ast.allocator);
            let new_consequent = match consequent {
                ast::Expression::ConditionalExpression(conditional_expression) => {
                    self.transform_conditional_expression(
                        &conditional_expression,
                        ctx,
                        inline,
                        true,
                    )
                    .1
                }
                ast::Expression::LogicalExpression(logical_expression) => {
                    self.transform_logical_expression(&logical_expression, ctx, inline, true)
                        .1
                }
                _ => consequent,
            };
            let alternate = conditional_expression.alternate.clone_in(ctx.ast.allocator);
            let new_alternate = match alternate {
                ast::Expression::ConditionalExpression(conditional_expression) => {
                    self.transform_conditional_expression(
                        &conditional_expression,
                        ctx,
                        inline,
                        true,
                    )
                    .1
                }
                ast::Expression::LogicalExpression(logical_expression) => {
                    self.transform_logical_expression(&logical_expression, ctx, inline, true)
                        .1
                }
                _ => alternate,
            };
            let (callee, short_circuit) = if inline {
                (
                    ctx.ast.expression_call(
                        SPAN,
                        memo.create_read_expression(ctx),
                        NONE,
                        ctx.ast.vec1(ctx.ast.argument_expression(
                            ast::Expression::ArrowFunctionExpression(ctx.ast.alloc(
                                ctx.ast.arrow_function_expression_with_scope_id(
                                    SPAN,
                                    true,
                                    false,
                                    NONE,
                                    ctx.ast.formal_parameters(
                                        SPAN,
                                        ast::FormalParameterKind::ArrowFormalParameters,
                                        ctx.ast.vec(),
                                        NONE,
                                    ),
                                    NONE,
                                    ctx.ast.function_body(
                                        SPAN,
                                        ctx.ast.vec(),
                                        ctx.ast.vec1(ctx.ast.statement_expression(SPAN, condition)),
                                    ),
                                    ctx.create_child_scope_of_current(ScopeFlags::Arrow),
                                ),
                            )),
                        )),
                        false,
                    ),
                    None,
                )
            } else {
                let identifier =
                    ctx.generate_uid_in_current_scope("_c$", SymbolFlags::FunctionScopedVariable);
                (
                    identifier.create_read_expression(ctx),
                    Some((identifier, condition)),
                )
            };
            (
                ctx.ast.expression_conditional(
                    SPAN,
                    ctx.ast
                        .expression_call(SPAN, callee, NONE, ctx.ast.vec(), false),
                    new_consequent,
                    new_alternate,
                ),
                short_circuit,
            )
        } else {
            (
                ctx.ast.expression_from_conditional(
                    conditional_expression.clone_in(ctx.ast.allocator),
                ),
                None,
            )
        };
        self.transform_short_circuit(memo, transformed, short_circuit, ctx, deep)
    }
}

impl<'a> TransformResult<'a> {
    fn create_template(
        self,
        config: &Config,
        traverse_ctx: &mut oxc_traverse::TraverseCtx<'a>,
        creation_ctx: &mut TemplateCreationCtx<'a>,
        wrap: bool,
    ) -> ast::Expression<'a> {
        match config.generate {
            OutputType::Dom => self.create_template_dom(config, traverse_ctx, creation_ctx, wrap),
        }
    }
}

pub struct TemplateCreationCtx<'a> {
    pub templates: Vec<Template<'a>>,
    pub imports: HashMap<(String, String), BoundIdentifier<'a>>,
}

pub struct Template<'a> {
    pub id: Atom<'a>,
    pub template: String,
    pub renderer: OutputType,
}

impl<'a> TemplateCreationCtx<'a> {
    pub fn register_import_method(
        &mut self,
        name: &str,
        module_name: &str,
        ctx: &mut TraverseCtx<'a>,
    ) -> BoundIdentifier<'a> {
        match self
            .imports
            .entry((name.to_owned(), module_name.to_owned()))
        {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry
                .insert(ctx.generate_uid_in_root_scope(&format!("_${}", name), SymbolFlags::Import))
                .clone(),
        }
    }

    fn get_leading_stmts(
        &self,
        module_name: &str,
        ctx: &mut TraverseCtx<'a>,
    ) -> Vec<ast::Statement<'a>> {
        let mut stmts = self.get_imports(ctx);

        if !self.templates.is_empty() {
            let (tmpl_fn, tmpl_fn_import) = self.get_template_fn(module_name, ctx);
            stmts.insert(0, tmpl_fn_import);
            stmts.push(self.get_template_decl(&tmpl_fn, ctx))
        }

        stmts
    }

    fn get_imports(&self, ctx: &mut TraverseCtx<'a>) -> Vec<ast::Statement<'a>> {
        self.imports
            .iter()
            .map(|((name, module_name), local)| {
                ctx.ast.statement_module_declaration(
                    ctx.ast.module_declaration_import_declaration(
                        SPAN,
                        Some(
                            ctx.ast
                                .vec1(ctx.ast.import_declaration_specifier_import_specifier(
                                    SPAN,
                                    ctx.ast.module_export_name_identifier_name(SPAN, name),
                                    local.create_binding_identifier(ctx),
                                    ast::ImportOrExportKind::Value,
                                )),
                        ),
                        ctx.ast.string_literal(SPAN, module_name),
                        NONE,
                        ast::ImportOrExportKind::Value,
                    ),
                )
            })
            .collect::<Vec<_>>()
    }

    fn get_template_fn(
        &self,
        module_name: &str,
        ctx: &mut TraverseCtx<'a>,
    ) -> (BoundIdentifier<'a>, ast::Statement<'a>) {
        let template_fn = ctx.generate_uid_in_root_scope("$template", SymbolFlags::Import);
        let binding_ident = template_fn.create_binding_identifier(ctx);

        (
            template_fn,
            ctx.ast.statement_module_declaration(
                ctx.ast.module_declaration_import_declaration(
                    SPAN,
                    Some(
                        ctx.ast
                            .vec1(ctx.ast.import_declaration_specifier_import_specifier(
                                SPAN,
                                ctx.ast.module_export_name_identifier_name(SPAN, "template"),
                                binding_ident,
                                ast::ImportOrExportKind::Value,
                            )),
                    ),
                    ctx.ast.string_literal(SPAN, module_name),
                    NONE,
                    ast::ImportOrExportKind::Value,
                ),
            ),
        )
    }

    fn get_template_decl(
        &self,
        template_fn: &BoundIdentifier<'a>,
        ctx: &mut TraverseCtx<'a>,
    ) -> ast::Statement<'a> {
        ctx.ast.statement_declaration(ctx.ast.declaration_variable(
            SPAN,
            ast::VariableDeclarationKind::Var,
            ctx.ast.vec_from_iter(self.templates.iter().map(|tmpl| {
                ctx.ast.variable_declarator(
                    SPAN,
                    ast::VariableDeclarationKind::Var,
                    ctx.ast.binding_pattern(
                        ctx.ast
                            .binding_pattern_kind_binding_identifier(SPAN, tmpl.id.clone()),
                        NONE,
                        false,
                    ),
                    Some(ctx.ast.expression_call(
                        SPAN,
                        template_fn.create_read_expression(ctx),
                        NONE,
                        ctx.ast.vec1(ast::Argument::TemplateLiteral(
                            ctx.ast.alloc_template_literal(
                                SPAN,
                                ctx.ast.vec1(ctx.ast.template_element(
                                    SPAN,
                                    true,
                                    ast::TemplateElementValue {
                                        raw: tmpl.template.clone().into_in(ctx.ast.allocator),
                                        cooked: None,
                                    },
                                )),
                                ctx.ast.vec(),
                            ),
                        )),
                        false,
                    )),
                    false,
                )
            })),
            false,
        ))
    }
}

#[cfg(test)]
mod transform_tests {
    use super::*;
    use oxc::{
        allocator::{Allocator, IntoIn},
        parser::Parser,
        semantic::SemanticBuilder,
        span::SourceType,
    };

    struct TestCase<'a> {
        source: &'static str,
        expected_id: Option<Atom<'a>>,
        expected_template: Option<String>,
        expected_exprs_len: usize,
        expected_text: bool,
    }

    #[test]
    fn test_transform_element() {
        let allocator = Allocator::default();

        let test_cases = vec![
            /* solidJS client side rendering result
                import { template as _$template } from "solid-js/web";
                var _tmpl$ = /*#__PURE__*/_$template(`<div class=test-class>Hello`); // <-
                const foo = _tmpl$();
            */
            TestCase {
                source: r#"<div class="test-class">Hello</div>"#,
                expected_id: Some("_el$2".into_in(&allocator)),
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
                expected_id: Some("_el$2".into_in(&allocator)),
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
                expected_id: Some("_el$2".into_in(&allocator)),
                expected_template: Some(r#"<span class=highlight>Text"#.to_string()),
                expected_exprs_len: 0,
                expected_text: false,
            },
        ];

        for case in test_cases {
            let source_type = SourceType::jsx();

            let parse_result = Parser::new(&allocator, case.source, source_type).parse();
            let program = parse_result.program;

            let semantic_result = SemanticBuilder::new()
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
                    let info = TransformInfo::default();
                    let mut transform = JsxTransform::new(config);

                    let result = transform.transform_element(jsx_element, &mut ctx, &info);

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
