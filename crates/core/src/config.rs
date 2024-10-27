pub struct Config {
    pub module_name: String,
    pub generate: OutputType,
    pub hydratable: bool,
    pub delegate_events: bool,
    pub delegated_events: Vec<String>,
    pub built_ins: Vec<String>,
    pub require_import_source: bool,
    pub wrap_conditionals: bool,
    pub omit_nested_closing_tags: bool,
    pub context_to_custom_elements: bool,
    pub static_marker: String,
    pub effect_wrapper: String,
    pub memo_wrapper: String,
    pub validate: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutputType {
    Dom,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            module_name: "dom".to_string(),
            generate: OutputType::Dom,
            hydratable: false,
            delegate_events: true,
            delegated_events: Vec::new(),
            built_ins: Vec::new(),
            require_import_source: false,
            wrap_conditionals: true,
            omit_nested_closing_tags: false,
            context_to_custom_elements: false,
            static_marker: "@once".to_string(),
            effect_wrapper: "effect".to_string(),
            memo_wrapper: "memo".to_string(),
            validate: true,
        }
    }
}
