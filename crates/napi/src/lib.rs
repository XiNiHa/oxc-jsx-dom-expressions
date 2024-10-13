use crate::config::Config;
use napi::bindgen_prelude::*;

use napi_derive::*;

pub mod config;

#[napi]
pub fn transform(source: String, config: Option<Config>) -> Result<String> {
    Ok(oxc_jsx_dom_expressions::transform(source, config.unwrap_or_default().into()).unwrap())
}
