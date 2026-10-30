//! The digest port, implemented with BLAKE3. A line's fingerprint is its item fingerprint, so a lock keys
//! a violation by its content and place: BLAKE3 over the line's text with its whitespace removed, a zero byte, and the
//! keyword and name of the item (function, method, type, impl, module, class, interface) that encloses the line, read
//! from indentation without parsing; the first eight bytes of the output, little-endian. The item
//! keywords of the common languages are read; `function`, a function bound to a `const`, a method `name(...) {` and `func`
//! with its receiver are read beside them, so the same text in two of those is two violations too.
//!
//! Neither a line number nor the path is read, so a violation keeps its fingerprint when lines are inserted above it,
//! when its function moves within the file, when an unrelated function is renamed, and when its file moves; editing
//! the line itself, or renaming its enclosing item, is a change to it. The same text in two functions is two
//! violations; two on byte-identical lines of one item share one, which the lock counts.

use super::constants::{
    ARROW, ASYNC, BINDINGS, BODY, CONTROL, FUNCTION, IMPL, ITEMS, METHOD, PUB_OPEN, QUALIFIERS,
    SPACED_ITEMS, SPACED_WHERE, WHERE,
};
use crate::domain::reports::types::Digest;
use crate::domain::violations::types::{Fingerprint, SourceText};
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::{
    Binding, Body, Declaration, Declares, DigestParts, Indent, ItemWord, LineFingerprints,
    LineText, Open, Reading, Words,
};

/// The BLAKE3 digest adapter.
#[derive(Debug)]
pub(crate) struct DigestBlake3Adapter;

impl DigestPort for DigestBlake3Adapter {
    fn lines(&self, text: &SourceText) -> LineFingerprints {
        // `split`, not `lines`: a file ending in a newline has one more, empty, line after it.
        let mut open: Vec<Open> = Vec::new();
        let mut prints = Vec::new();
        for line in text.0.split('\n').map(LineText) {
            let content = LineText(line.0.trim());
            if content.0.is_empty() {
                prints.push(of_line(line, enclosing(&open, Indent(usize::MAX))));
                continue;
            }
            let indent = indentation(line);
            if reading(content) == Reading::Continues {
                // The rest of a declaration begun above, such as a multi-line signature's `) -> Answer {`, its
                // `where` clause or the `{` after one: it closes what is deeper and opens nothing. A closing brace
                // closes the body that opened at its own indentation too, after its line is read as inside it.
                while open.pop_if(|scope| scope.indent > indent).is_some() {}
                prints.push(of_line(line, enclosing(&open, indent)));
                if content.0.starts_with('}') {
                    while open.pop_if(|scope| scope.indent >= indent).is_some() {}
                }
                continue;
            }
            let declares = declared(content);
            if declares == Declares::Nothing {
                // A line that declares nothing is inside a body that opened at its own indentation, a body written
                // without indenting its lines: the item stays open for it.
                while open
                    .pop_if(|scope| {
                        scope.indent > indent
                            || (scope.indent == indent && scope.body == Body::Absent)
                    })
                    .is_some()
                {}
                prints.push(of_line(line, enclosing(&open, Indent(indent.0 + 1))));
            } else {
                while open.pop_if(|scope| scope.indent >= indent).is_some() {}
                prints.push(of_line(line, enclosing(&open, indent)));
            }
            open.push(Open {
                indent,
                declares,
                body: if content.0.ends_with('{') {
                    Body::Opens
                } else {
                    Body::Absent
                },
            });
        }
        LineFingerprints(prints)
    }

    fn digest(&self, parts: DigestParts<'_>) -> Digest {
        let mut hasher = blake3::Hasher::new();
        let fed = parts
            .0
            .iter()
            .fold(&mut hasher, |hasher, part| hasher.update(part).update(&[0]));
        // The first 16 bytes, as 32 hex digits.
        Digest(
            fed.finalize()
                .to_hex()
                .chars()
                .take(32)
                .collect::<String>()
                .into(),
        )
    }
}

/// What the nearest open line less indented than `indent` declares.
fn enclosing(open: &[Open], indent: Indent) -> Declares {
    open.iter()
        .rev()
        .filter(|scope| scope.indent < indent)
        .find_map(|scope| match &scope.declares {
            Declares::Item(item) => Some(Declares::Item(item.clone())),
            Declares::Nothing => None,
        })
        .unwrap_or(Declares::Nothing)
}

/// Whether a line's trimmed `content` continues a declaration rather than beginning one: it opens with a bracket
/// that closes a span or opens a body, or it is a `where` clause.
fn reading(content: LineText<'_>) -> Reading {
    let named = |next| char::is_alphanumeric(next) || next == '_';
    let continues = content.0.starts_with([')', ']', '}', '{'])
        || content
            .0
            .strip_prefix(WHERE)
            .is_some_and(|rest| !rest.starts_with(named));
    if continues {
        Reading::Continues
    } else {
        Reading::Begins
    }
}

/// The number of whitespace characters a line starts with.
fn indentation(line: LineText<'_>) -> Indent {
    Indent(
        line.0
            .chars()
            .take_while(|character| character.is_whitespace())
            .count(),
    )
}

/// The first word of `text` and what follows it.
fn first_word(text: LineText<'_>) -> Words<'_> {
    let end = text.0.find(char::is_whitespace).unwrap_or(text.0.len());
    // `find` answers a character boundary, so the split always holds.
    let (word, rest) = text.0.split_at_checked(end).unwrap_or_default();
    Words {
        word: LineText(word),
        rest: LineText(rest),
    }
}

/// The name `text` opens with, as an item name is read: letters, digits and `_`.
fn leading_name(text: LineText<'_>) -> LineText<'_> {
    let named = |next| char::is_alphanumeric(next) || next == '_';
    LineText(
        text.0
            .split_once(|next| !named(next))
            .map_or(text.0, |(name, _)| name),
    )
}

/// The name `text` opens with where a name may also hold a `$`.
fn script_name(text: LineText<'_>) -> LineText<'_> {
    let named = |next| char::is_alphanumeric(next) || next == '_' || next == '$';
    LineText(
        text.0
            .split_once(|next| !named(next))
            .map_or(text.0, |(name, _)| name),
    )
}

/// What follows the parenthesis `text` opens with and its match, when it closes.
fn after_parentheses(text: LineText<'_>) -> Option<LineText<'_>> {
    let mut depth = 0_usize;
    for (at, character) in text.0.char_indices() {
        if character == '(' {
            depth += 1;
        }
        if character == ')' {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return text.0.get(at + 1..).map(LineText);
            }
        }
    }
    None
}

/// What `content` declares: its keyword after any qualifiers, and the name after it. An `impl` is named by
/// everything up to its body or `where` clause, since its trait and type are its name.
fn declared(content: LineText<'_>) -> Declares {
    let mut rest = content;
    let mut bound = Binding::Unbound;
    loop {
        let Words { word, rest: after } = first_word(rest);
        let qualifier =
            QUALIFIERS.contains(&word.0) || (word.0.starts_with(PUB_OPEN) && word.0.ends_with(')'));
        let binding = BINDINGS.contains(&word.0);
        if !qualifier && !binding {
            break;
        }
        if binding {
            bound = Binding::Bound;
        }
        rest = LineText(after.0.trim_start());
    }
    item(rest)
        .or_else(|| spaced_item(rest))
        .or_else(|| match bound {
            Binding::Bound => bound_function(rest),
            Binding::Unbound => None,
        })
        .or_else(|| method(rest))
        .unwrap_or(Declares::Nothing)
}

/// The declaration `rest` opens with one of the item keywords.
fn item(rest: LineText<'_>) -> Option<Declares> {
    let named = |next| char::is_alphanumeric(next) || next == '_';
    let (keyword, after) = ITEMS.iter().find_map(|keyword| {
        rest.0
            .strip_prefix(*keyword)
            .filter(|after| after.is_empty() || after.starts_with(|next| !named(next)))
            .map(|after| (keyword, after.trim_start()))
    })?;
    let name = if *keyword == IMPL {
        // Up to the body or the `where` clause, whichever comes first.
        let end = [after.find('{'), after.find(SPACED_WHERE)]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(after.len());
        after.get(..end).unwrap_or(after).trim_end()
    } else {
        leading_name(LineText(after)).0
    };
    Some(Declares::Item(Declaration {
        keyword: ItemWord((*keyword).into()),
        name: ItemWord(name.into()),
    }))
}

/// The declaration `rest` opens with a keyword followed by whitespace: `function name`, `interface Name`, `func name`,
/// or a method named by its receiver and its name, `func (s *Server) Start`.
fn spaced_item(rest: LineText<'_>) -> Option<Declares> {
    let Words { word, rest: after } = first_word(rest);
    let keyword = SPACED_ITEMS.iter().find(|keyword| **keyword == word.0)?;
    let after = after.0.trim_start();
    let name = if after.starts_with('(') {
        let receiver_end = after.len() - after_parentheses(LineText(after))?.0.len();
        let receiver = after.get(..receiver_end)?;
        let method = script_name(LineText(after.get(receiver_end..)?.trim_start()));
        format!("{receiver}{}", method.0)
    } else {
        script_name(LineText(after.trim_start_matches('*').trim_start()))
            .0
            .to_owned()
    };
    Some(Declares::Item(Declaration {
        keyword: ItemWord((*keyword).into()),
        name: ItemWord(name.into()),
    }))
}

/// The function a `const`, `let` or `var` binds: `name = function`, `name = (...) =>` or `name = x =>`, any of them
/// after `async`, a type annotation allowed before the `=`.
fn bound_function(rest: LineText<'_>) -> Option<Declares> {
    let name = script_name(rest);
    if name.0.is_empty() {
        return None;
    }
    let (_, value) = rest.0.get(name.0.len()..)?.split_once('=')?;
    let arrow = |after: LineText<'_>| after.0.trim_start().starts_with(ARROW);
    let mut value = value.trim_start();
    if script_name(LineText(value)).0 == ASYNC {
        value = value.get(ASYNC.len()..)?.trim_start();
    }
    let word = script_name(LineText(value)).0;
    let function = word == FUNCTION
        || (value.starts_with('(')
            && after_parentheses(LineText(value)).is_some_and(|after| {
                let after = after.0.trim_start();
                arrow(LineText(after)) || (after.starts_with(':') && after.contains(ARROW))
            }))
        || (!word.is_empty()
            && value
                .get(word.len()..)
                .is_some_and(|tail| arrow(LineText(tail))));
    function.then(|| {
        Declares::Item(Declaration {
            keyword: ItemWord(BINDINGS[0].into()),
            name: ItemWord(name.0.into()),
        })
    })
}

/// The method `rest` declares in a class or an object: `name(parameters) {`, a return type allowed before the body;
/// a control word, or a call whose arguments hold a string or a call, declares none.
fn method(rest: LineText<'_>) -> Option<Declares> {
    let name = script_name(LineText(rest.0.trim_start_matches(['#', '*']))).0;
    if name.is_empty() || CONTROL.contains(&name) {
        return None;
    }
    let parameters = rest.0.get(rest.0.find(name)? + name.len()..)?;
    if !parameters.starts_with('(') {
        return None;
    }
    let after = after_parentheses(LineText(parameters))?.0;
    let inside = parameters.get(1..parameters.len() - after.len() - 1)?;
    let after = after.trim();
    let body = after == BODY || (after.starts_with(':') && after.ends_with(BODY));
    (body && !inside.contains(['(', '"', '\'', '`'])).then(|| {
        Declares::Item(Declaration {
            keyword: ItemWord(METHOD.into()),
            name: ItemWord(name.into()),
        })
    })
}

/// The fingerprint of one line inside what `declares`.
fn of_line(line: LineText<'_>, declares: Declares) -> Fingerprint {
    let mut hasher = blake3::Hasher::new();
    // The zero byte separates the line from its item, so text cannot slide between them and collide.
    let separated = feed(&mut hasher, line).update(&[0]);
    let fed = match declares {
        Declares::Item(Declaration { keyword, name }) => {
            feed(feed(separated, LineText(&keyword.0)), LineText(&name.0))
        }
        Declares::Nothing => separated,
    };
    let mut prefix = [0_u8; 8];
    fed.finalize_xof().fill(&mut prefix);
    Fingerprint(u64::from_le_bytes(prefix))
}

/// Feeds `text`'s characters other than whitespace to `hasher`, and hands it back for what follows.
fn feed<'hasher>(
    hasher: &'hasher mut blake3::Hasher,
    text: LineText<'_>,
) -> &'hasher mut blake3::Hasher {
    let mut buffer = [0_u8; 4];
    text.0
        .chars()
        .filter(|character| !character.is_whitespace())
        .fold(hasher, |hasher, character| {
            hasher.update(character.encode_utf8(&mut buffer).as_bytes())
        })
}
