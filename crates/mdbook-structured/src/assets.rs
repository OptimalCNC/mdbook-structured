#[allow(dead_code)]
pub(crate) const STRUCTURED_CSS: &'static [u8] = include_bytes!("../assets/mdbook-structured.css");

#[cfg(test)]
mod tests {
    use super::STRUCTURED_CSS;

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
}
