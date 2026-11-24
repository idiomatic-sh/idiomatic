//! The query glob translated to the byte regex used by both plan consumers.
pub(super) fn glob_pattern(g: &str) -> Option<String> {
    // Globs match the complete published value, including line terminators.
    let mut s = String::from("(?s)^");
    if !g.is_ascii() || g.contains(['[', '?', '\\']) || g.contains("**/") {
        return None;
    }
    let mut depth = 0usize;
    for c in g.chars() {
        match c {
            '*' => s.push_str(".*"),
            '?' => s.push('.'),
            '{' => { depth += 1; s.push_str("(?:"); }
            '}' if depth > 0 => { depth -= 1; s.push(')'); }
            '}' => return None,
            ',' if depth > 0 => s.push('|'),
            c => s.push_str(&regex::escape(&c.to_string())),
        }
    }
    s.push('$');
    (depth == 0).then_some(s)
}
