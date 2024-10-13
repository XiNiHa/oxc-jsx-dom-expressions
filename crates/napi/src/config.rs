use napi_derive::*;
use oxc_jsx_dom_expressions as core;

#[napi(object)]
#[derive(Default)]
pub struct Config {
    pub module_name: Option<String>,
    pub generate: Option<OutputType>,
    pub hydratable: Option<bool>,
    pub delegate_events: Option<bool>,
    pub delegated_events: Option<Vec<String>>,
    pub built_ins: Option<Vec<String>>,
    pub require_import_source: Option<bool>,
    pub wrap_conditionals: Option<bool>,
    pub omit_nested_closing_tags: Option<bool>,
    pub context_to_custom_elements: Option<bool>,
    pub static_marker: Option<String>,
    pub effect_wrapper: Option<String>,
    pub memo_wrapper: Option<String>,
    pub validate: Option<bool>,
}

#[napi(string_enum = "lowercase")]
pub enum OutputType {
    Dom,
}

impl From<Config> for core::Config {
    fn from(options: Config) -> Self {
        let default = core::Config::default();
        Self {
            module_name: options.module_name.unwrap_or(default.module_name),
            generate: options
                .generate
                .map(|v| v.into())
                .unwrap_or(default.generate),
            hydratable: options.hydratable.unwrap_or(default.hydratable),
            delegate_events: options.delegate_events.unwrap_or(default.delegate_events),
            delegated_events: options.delegated_events.unwrap_or(default.delegated_events),
            built_ins: options.built_ins.unwrap_or(default.built_ins),
            require_import_source: options
                .require_import_source
                .unwrap_or(default.require_import_source),
            wrap_conditionals: options
                .wrap_conditionals
                .unwrap_or(default.wrap_conditionals),
            omit_nested_closing_tags: options
                .omit_nested_closing_tags
                .unwrap_or(default.omit_nested_closing_tags),
            context_to_custom_elements: options
                .context_to_custom_elements
                .unwrap_or(default.context_to_custom_elements),
            static_marker: options.static_marker.unwrap_or(default.static_marker),
            effect_wrapper: options.effect_wrapper.unwrap_or(default.effect_wrapper),
            memo_wrapper: options.memo_wrapper.unwrap_or(default.memo_wrapper),
            validate: options.validate.unwrap_or(default.validate),
        }
    }
}

impl From<OutputType> for core::OutputType {
    fn from(options: OutputType) -> Self {
        match options {
            OutputType::Dom => core::OutputType::Dom,
        }
    }
}
