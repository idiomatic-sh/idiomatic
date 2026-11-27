//! The query glob translated to the byte regex used by both plan consumers.
pub(crate) fn glob_pattern(g: &str) -> Option<String> {
    thread_local! {
        static TRANSLATED: std::cell::RefCell<std::collections::HashMap<Box<str>, Option<String>>> = Default::default();
    }
    TRANSLATED.with(|cache| {
        if let Some(known) = cache.borrow().get(g) {
            return known.clone();
        }
        let translated = translate(g);
        cache.borrow_mut().insert(g.into(), translated.clone());
        translated
    })
}

pub(crate) fn translate(g: &str) -> Option<String> {
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
            c => {
                if "\\.+*?()|[]{}^$#&-~".contains(c) {
                    s.push('\\');
                }
                s.push(c);
            }
        }
    }
    s.push('$');
    (depth == 0).then_some(s)
}
