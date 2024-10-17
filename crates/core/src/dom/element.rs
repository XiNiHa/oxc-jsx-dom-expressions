use oxc::ast::ast;
use oxc_traverse::TraverseCtx;

use crate::shared::transform::{JsxTransform, TransformInfo, TransformResult};

impl<'a> JsxTransform {
    pub fn transform_element_dom(
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
                    skip_template: false,
                };
            }
        };

        let attributes = self.generate_attributes_dom(&el.opening_element.attributes);
        let child_templates = self.generate_child_templates_dom(&el.children, ctx);

        // TODO
        let template = format!("<{}{}>{}", tag_name, attributes, child_templates);

        TransformResult {
            id: None,
            template: Some(template),
            exprs: ctx.ast.vec(),
            text: false,
            skip_template: false,
        }
    }

    /// generate attributes string without quotes around values
    fn generate_attributes_dom(&self, attrs: &[ast::JSXAttributeItem<'a>]) -> String {
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
    fn generate_child_templates_dom(
        &self,
        children: &[ast::JSXChild<'a>],
        ctx: &mut TraverseCtx<'a>,
    ) -> String {
        let info = TransformInfo::default();
        children
            .iter()
            .filter_map(|child| self.transform_node(child, ctx, &info)?.template)
            .collect::<String>()
    }
}
