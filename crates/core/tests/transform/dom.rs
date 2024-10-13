use oxc_jsx_dom_expressions::*;
use pretty_assertions::assert_eq;

tests_macros::gen_tests! {"tests/transform/specs/dom/*.js", crate::transform::dom::run_test, "module"}

fn run_test(input_path: &'static str, expected_path: &'static str, _: &str, _: &str) {
    let source = std::fs::read_to_string(input_path).unwrap();
    let expected = std::fs::read_to_string(expected_path).unwrap();

    assert_eq!(
        transform(
            source,
            Config {
                module_name: "r-dom".to_string(),
                built_ins: vec!["For".to_string(), "Show".to_string()],
                generate: OutputType::Dom,
                wrap_conditionals: true,
                context_to_custom_elements: true,
                static_marker: "@once".to_string(),
                require_import_source: false,
                ..Default::default()
            }
        )
        .unwrap(),
        super::roundtrip(&expected)
    );
}
