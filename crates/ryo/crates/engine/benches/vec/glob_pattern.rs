//! The query glob translated to the byte regex used by both plan consumers.
pub(super) fn glob_pattern(g: &str) -> String {
    let mut s = String::from("^");
    for c in g.chars() {
        match c {
            '*' => s.push_str(".*"),
            '?' => s.push('.'),
            '{' => s.push_str("(?:"),
            '}' => s.push(')'),
            ',' => s.push('|'),
            c => s.push_str(&regex::escape(&c.to_string())),
        }
    }
    s.push('$');
    s
}
