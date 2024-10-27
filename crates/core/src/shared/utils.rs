use oxc::{
    ast::{ast, visit::walk, Visit},
    span::Atom,
};

pub fn jsx_text_to_str(t: &Atom) -> String {
    let mut buf = String::new();
    let replaced = t.replace('\r', "").replace('\t', " ");
    let mut lines = replaced.lines().enumerate().peekable();

    while let Some((i, mut line)) = lines.next() {
        if line.is_empty() {
            continue;
        }
        if i != 0 {
            line = line.trim_start_matches(' ');
        }
        if lines.peek().is_some() {
            line = line.trim_end_matches(' ');
        }
        if line.is_empty() {
            continue;
        }
        if i != 0 && !buf.is_empty() {
            buf.push(' ')
        }
        buf.push_str(line);
    }
    buf
}

trait IsDynamic {
    fn walk<'a>(&'a self, visitor: &mut impl Visit<'a>);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DynamicChecker {
    check_member: bool,
    check_tags: bool,
    check_call_expressions: bool,
    native: bool,
}

impl DynamicChecker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_member(self, yes: bool) -> Self {
        Self {
            check_member: yes,
            ..self
        }
    }

    pub fn check_tags(self, yes: bool) -> Self {
        Self {
            check_tags: yes,
            ..self
        }
    }

    pub fn check_call_expressions(self, yes: bool) -> Self {
        Self {
            check_call_expressions: yes,
            ..self
        }
    }

    pub fn native(self, yes: bool) -> Self {
        Self {
            native: yes,
            ..self
        }
    }

    pub fn check(&self, expression: &impl IsDynamic) -> bool {
        let mut visitor = DynamicVisitor {
            is_dynamic: false,
            checker: self,
            current_skip_span: ast::Span::empty(0),
        };
        expression.walk(&mut visitor);
        visitor.is_dynamic
    }
}

struct DynamicVisitor<'a> {
    is_dynamic: bool,
    checker: &'a DynamicChecker,
    current_skip_span: ast::Span,
}

impl Visit<'_> for DynamicVisitor<'_> {
    fn visit_function(&mut self, it: &ast::Function<'_>, _flags: oxc::semantic::ScopeFlags) {
        if self.current_skip_span.contains_inclusive(it.span) {
            return;
        }
        self.current_skip_span = it.span;
    }

    fn visit_call_expression(&mut self, it: &ast::CallExpression<'_>) {
        if self.current_skip_span.contains_inclusive(it.span) {
            return;
        }
        if self.checker.check_call_expressions {
            self.is_dynamic = true;
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ast::ComputedMemberExpression<'_>) {
        if self.current_skip_span.contains_inclusive(it.span) {
            return;
        }
        if self.checker.check_member {
            self.is_dynamic = true;
        }
    }

    fn visit_static_member_expression(&mut self, it: &ast::StaticMemberExpression<'_>) {
        if self.current_skip_span.contains_inclusive(it.span) {
            return;
        }
        if self.checker.check_member {
            self.is_dynamic = true;
        }
    }

    fn visit_private_field_expression(&mut self, it: &ast::PrivateFieldExpression<'_>) {
        if self.current_skip_span.contains_inclusive(it.span) {
            return;
        }
        if self.checker.check_member {
            self.is_dynamic = true;
        }
    }

    fn visit_spread_element(&mut self, it: &ast::SpreadElement<'_>) {
        if self.current_skip_span.contains_inclusive(it.span) {
            return;
        }
        if self.checker.check_member {
            self.is_dynamic = true;
        }
    }

    fn visit_binary_expression(&mut self, it: &ast::BinaryExpression<'_>) {
        if self.current_skip_span.contains_inclusive(it.span) {
            return;
        }
        if self.checker.check_member && it.operator.is_in() {
            // TODO: exclude (namespace import).property
            self.is_dynamic = true;
        }
    }

    fn visit_jsx_element(&mut self, it: &ast::JSXElement<'_>) {
        if self.current_skip_span.contains_inclusive(it.span) {
            return;
        }
        if self.checker.check_tags {
            self.is_dynamic = true;
        }
    }
}

impl IsDynamic for ast::Expression<'_> {
    fn walk<'a>(&'a self, visitor: &mut impl Visit<'a>) {
        walk::walk_expression(visitor, self);
    }
}

impl IsDynamic for ast::JSXExpression<'_> {
    fn walk<'a>(&'a self, visitor: &mut impl Visit<'a>) {
        walk::walk_jsx_expression(visitor, self);
    }
}
