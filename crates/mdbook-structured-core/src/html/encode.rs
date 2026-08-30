pub(super) fn push_encoded_text(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '{' => output.push_str("&#123;"),
            '}' => output.push_str("&#125;"),
            '\r' => output.push_str("&#13;"),
            '\n' => output.push_str("&#10;"),
            _ => output.push(character),
        }
    }
}

pub(super) fn push_encoded_attribute(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '{' => output.push_str("&#123;"),
            '}' => output.push_str("&#125;"),
            '\'' => output.push_str("&#39;"),
            '"' => output.push_str("&quot;"),
            '\r' => output.push_str("&#13;"),
            '\n' => output.push_str("&#10;"),
            _ => output.push(character),
        }
    }
}
