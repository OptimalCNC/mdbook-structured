#[allow(dead_code)]
pub(crate) const STRUCTURED_CSS: &'static [u8] = include_bytes!("../assets/mdbook-structured.css");

#[allow(dead_code)]
pub(crate) const STRUCTURED_JS: &'static [u8] = include_bytes!("../assets/mdbook-structured.js");

#[cfg(test)]
mod tests {
    use super::{STRUCTURED_CSS, STRUCTURED_JS};

    #[test]
    fn css_exposes_the_frozen_structured_hooks() {
        let css = std::str::from_utf8(STRUCTURED_CSS).expect("embedded CSS must be UTF-8");

        for selector in [
            ".structured-document",
            "details[data-structured-container]",
            "[data-structured-node=\"mapping\"]",
            "[data-structured-node=\"sequence\"]",
            "[data-structured-node=\"string\"]",
            "[data-structured-node=\"number\"]",
            "[data-structured-node=\"boolean\"]",
            "[data-structured-node=\"null\"]",
            "[data-structured-label=\"key\"]",
            "[data-structured-label=\"index\"]",
            "[data-structured-value]",
            "[data-structured-empty=\"key\"]",
            "[data-structured-empty=\"string\"]",
            "[data-structured-action]",
            "details[data-structured-original-source]",
            "code[data-structured-format]",
        ] {
            assert!(css.contains(selector), "missing CSS selector: {selector}");
        }

        for variable in ["--bg", "--fg", "--links", "--sidebar-bg", "--theme-hover"] {
            assert!(
                css.contains(variable),
                "missing mdBook variable: {variable}"
            );
        }

        assert!(css.contains("white-space: pre-wrap"));
        assert!(css.contains("overflow-wrap: anywhere"));
        assert!(css.contains("content: \" (empty)\""));
        assert!(!css.contains("@media print"));
    }

    #[test]
    fn css_uses_distinct_theme_inputs_for_scalar_colors() {
        let css = std::str::from_utf8(STRUCTURED_CSS).expect("embedded CSS must be UTF-8");
        let root = declaration_block(css, ".structured-document");
        let properties = [
            "--structured-string",
            "--structured-number",
            "--structured-boolean",
            "--structured-null",
        ];
        let values = properties.map(|property| declaration_value(root, property));
        let distinct_values = values.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(distinct_values.len(), properties.len());

        let theme_inputs = values.map(primary_theme_input);
        let distinct_theme_inputs = theme_inputs
            .iter()
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(distinct_theme_inputs.len(), properties.len());
    }

    #[test]
    fn css_action_controls_include_border_box_sizing() {
        let css = std::str::from_utf8(STRUCTURED_CSS).expect("embedded CSS must be UTF-8");
        let action = declaration_block(css, "[data-structured-action]");

        assert!(action.contains("box-sizing: border-box"));
    }

    #[test]
    fn js_references_only_the_stable_disclosure_hooks() {
        let js = std::str::from_utf8(STRUCTURED_JS).expect("embedded JavaScript must be UTF-8");

        for hook in [
            ".structured-document",
            "[data-structured-action]",
            "details[data-structured-container]",
        ] {
            assert!(js.contains(hook), "missing JavaScript hook: {hook}");
        }

        for storage_identifier in ["cookie", "localStorage", "sessionStorage", "indexedDB"] {
            assert!(
                !js.contains(storage_identifier),
                "JavaScript must not use storage: {storage_identifier}"
            );
        }
    }

    fn declaration_block<'a>(css: &'a str, selector: &str) -> &'a str {
        let selector_start = css
            .find(selector)
            .unwrap_or_else(|| panic!("missing selector: {selector}"));
        let opening = css[selector_start..]
            .find('{')
            .map(|offset| selector_start + offset)
            .expect("selector must open a declaration block");
        let closing = css[opening..]
            .find('}')
            .map(|offset| opening + offset)
            .expect("declaration block must close");
        &css[opening + 1..closing]
    }

    fn declaration_value<'a>(block: &'a str, property: &str) -> &'a str {
        let declaration = block
            .lines()
            .find(|line| line.trim_start().starts_with(property))
            .unwrap_or_else(|| panic!("missing declaration: {property}"));
        declaration
            .split_once(':')
            .map(|(_, value)| value.trim().trim_end_matches(';'))
            .expect("declaration must contain a colon")
    }

    fn primary_theme_input(value: &str) -> &str {
        let variable_start = value.find("var(").expect("color must use a theme variable");
        let arguments = &value[variable_start + 4..];
        let comma = arguments
            .find(',')
            .expect("theme variable must include a fallback");
        arguments[..comma].trim()
    }
}
