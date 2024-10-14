use oxc::{
    ast::{ast, NONE},
    semantic::{ScopeFlags, ScopeId, SymbolFlags},
    span::{Atom, SPAN},
};
use oxc_traverse::{Ancestor, Traverse};

pub(crate) struct ThisToSelfTransform<'a> {
    jsx_depth: usize,
    current_parent: Option<ScopeId>,
    current_self_name: Option<Atom<'a>>,
    bindings: Vec<SelfBinding<'a>>,
}

struct SelfBinding<'a> {
    scope_id: ScopeId,
    self_name: Atom<'a>,
}

impl<'a> ThisToSelfTransform<'a> {
    pub(crate) fn new() -> Self {
        Self {
            jsx_depth: 0,
            current_parent: None,
            current_self_name: None,
            bindings: Vec::new(),
        }
    }

    fn belonging_function(
        ctx: &mut oxc_traverse::TraverseCtx<'_>,
        include_arrows: bool,
    ) -> Option<ScopeId> {
        ctx.ancestors().find_map(|a| match a {
            Ancestor::FunctionBody(f) => f.scope_id().get(),
            Ancestor::ArrowFunctionExpressionBody(f) if include_arrows => f.scope_id().get(),
            _ => None,
        })
    }

    fn self_name(&mut self, ctx: &mut oxc_traverse::TraverseCtx<'a>) -> Atom<'a> {
        match &self.current_self_name {
            Some(name) => name.clone(),
            None => {
                let name = ctx
                    .generate_uid_in_current_scope("self$", SymbolFlags::ConstVariable)
                    .name;
                self.current_self_name = Some(name.clone());
                name
            }
        }
    }

    fn make_self_binding_stmt(
        name: Atom<'a>,
        ctx: &mut oxc_traverse::TraverseCtx<'a>,
    ) -> ast::Statement<'a> {
        ctx.ast.statement_declaration(ctx.ast.declaration_variable(
            SPAN,
            ast::VariableDeclarationKind::Const,
            ctx.ast.vec1(ctx.ast.variable_declarator(
                SPAN,
                ast::VariableDeclarationKind::Const,
                ctx.ast.binding_pattern(
                    ctx.ast.binding_pattern_kind_binding_identifier(SPAN, name),
                    NONE,
                    false,
                ),
                Some(ctx.ast.expression_this(SPAN)),
                false,
            )),
            false,
        ))
    }
}

impl<'a> Traverse<'a> for ThisToSelfTransform<'a> {
    fn enter_expression(
        &mut self,
        node: &mut ast::Expression<'a>,
        ctx: &mut oxc_traverse::TraverseCtx<'a>,
    ) {
        use ast::Expression::*;

        match node {
            ThisExpression(this) => {
                let current = ThisToSelfTransform::belonging_function(ctx, false);
                if current == self.current_parent {
                    let self_name = self.self_name(ctx);
                    *node = ctx
                        .ast
                        .expression_identifier_reference(this.span, self_name);
                }
            }
            JSXElement(_) => {
                if self.jsx_depth == 0 {
                    self.current_parent = ThisToSelfTransform::belonging_function(ctx, true);
                }
                self.jsx_depth += 1;
            }
            _ => {}
        }
    }

    fn exit_expression(
        &mut self,
        node: &mut ast::Expression<'a>,
        ctx: &mut oxc_traverse::TraverseCtx<'a>,
    ) {
        use ast::Expression::*;

        if let JSXElement(_) = node {
            self.jsx_depth -= 1;

            if self.jsx_depth == 0 {
                if let Some(self_name) = &self.current_self_name {
                    match self.current_parent {
                        None => {
                            let arrow_fn = ctx.ast.arrow_function_expression(
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
                                    ctx.ast.vec1(ThisToSelfTransform::make_self_binding_stmt(
                                        self_name.clone(),
                                        ctx,
                                    )),
                                ),
                            );
                            arrow_fn
                                .scope_id
                                .set(Some(ctx.create_child_scope_of_current(ScopeFlags::Arrow)));
                            *node = ctx.ast.expression_call(
                                SPAN,
                                ctx.ast.expression_parenthesized(
                                    SPAN,
                                    ctx.ast.expression_from_arrow_function(arrow_fn),
                                ),
                                NONE,
                                ctx.ast.vec(),
                                false,
                            );
                        }
                        Some(parent) => {
                            self.bindings.push(SelfBinding {
                                scope_id: parent,
                                self_name: self_name.clone(),
                            });
                        }
                    }
                }

                self.current_parent = None;
                self.current_self_name = None;
            }
        }
    }

    fn enter_jsx_member_expression(
        &mut self,
        node: &mut ast::JSXMemberExpression<'a>,
        ctx: &mut oxc_traverse::TraverseCtx<'a>,
    ) {
        if self.jsx_depth == 0 {
            return;
        }

        if let ast::JSXMemberExpressionObject::ThisExpression(_) = node.object {
            let self_ref = self.self_name(ctx);
            node.object = ctx
                .ast
                .jsx_member_expression_object_identifier_reference(SPAN, self_ref);
        }
    }

    fn exit_function_body(
        &mut self,
        node: &mut ast::FunctionBody<'a>,
        ctx: &mut oxc_traverse::TraverseCtx<'a>,
    ) {
        let scope_id = ctx.current_scope_id();
        self.bindings.retain(|b| {
            if b.scope_id == scope_id {
                let stmt = ThisToSelfTransform::make_self_binding_stmt(b.self_name.clone(), ctx);
                node.statements.insert(0, stmt);
                false
            } else {
                true
            }
        });
    }

    fn exit_arrow_function_expression(
        &mut self,
        node: &mut ast::ArrowFunctionExpression<'a>,
        ctx: &mut oxc_traverse::TraverseCtx<'a>,
    ) {
        if node.expression && node.body.statements.len() > 1 {
            if let Some(ast::Statement::ExpressionStatement(_)) = node.body.statements.last() {
                let mut expr = match node.body.statements.pop() {
                    Some(ast::Statement::ExpressionStatement(e)) => e,
                    _ => unreachable!(),
                };
                node.body.statements.push(
                    ctx.ast
                        .statement_return(SPAN, Some(ctx.ast.move_expression(&mut expr.expression))),
                );
                node.expression = false;
            }
        }
    }
}
