use oxc_traverse::Traverse;

use crate::Config;

pub struct JsxTransform {
    config: Config,
}

impl JsxTransform {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl<'a> Traverse<'a> for JsxTransform {}
