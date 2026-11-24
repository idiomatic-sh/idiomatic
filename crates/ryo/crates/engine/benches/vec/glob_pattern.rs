//! Only literal, star and single-character globs have this regular-expression
//! representation. The caller uses the query language's matcher for its other forms.
pub(super) fn glob_pattern(pattern: &str) -> Option<String> {
    if pattern.contains(['\\', '[', '{', '}']) || pattern.contains("**/") {
        return None;
    }
    let mut regex = String::from("(?s)^");
    for c in pattern.chars() {
        match c {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            c => regex.push_str(&regex::escape(&c.to_string())),
        }
    }
    regex.push('$');
    Some(regex)
}
