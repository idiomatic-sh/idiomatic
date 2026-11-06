//! Test support split from adapter_test: Shape, shape, Line and more.

use super::adapter::SyntaxCssparserAdapter;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_name, kind_of};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::service::mirror;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::SourcePath;
use ryo::languages::{
    ByteSlice, Elements, Position, Symbols, children as children_at, end_at, kind_at, start_at,
    table_builder,
};

/// A node or a token of a tree, by its kind's name, with what it holds under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct Shape {
    /// The kind's name.
    label: String,
    /// What it holds, in order.
    children: Vec<Shape>,
}

pub(in super::super) fn shape(label: &str, children: Vec<Shape>) -> Shape {
    Shape {
        label: label.to_owned(),
        children,
    }
}

/// A line of the tool's print and the lines indented under it.
pub(in super::super) struct Line {
    head: String,
    below: Vec<Line>,
}

/// The standard's token that a `literal` of tinycss2 is, or the two delim-tokens of an attribute matcher.
pub(in super::super) fn literal(value: &str) -> Vec<Shape> {
    let one = |name: &str| vec![shape(name, Vec::new())];
    match value {
        ":" => one("colon-token"),
        ";" => one("semicolon-token"),
        "," => one("comma-token"),
        "<!--" => one("cdo-token"),
        "-->" => one("cdc-token"),
        "~=" | "|=" | "^=" | "$=" | "*=" | "||" => {
            vec![shape("delim-token", Vec::new()); 2]
        }
        _ => one("delim-token"),
    }
}

/// What a line of the print is in the standard's names, with what is under it: the nodes of the parsing results and the tokens of
/// the tokenizer. White space and comments are dropped, a rule's block flattened into the rule, a declaration given its name and
/// colon and its `!important` as the one token `important`.
pub(in super::super) fn standard_of(line: &Line) -> Vec<Shape> {
    let (word, argument) = line.head.split_once(' ').unwrap_or((&line.head, ""));
    let below = || -> Vec<Shape> { line.below.iter().flat_map(standard_of).collect() };
    let first = |name: &str| {
        let mut children = vec![shape(name, Vec::new())];
        children.extend(below());
        children
    };
    match (word, argument) {
        ("whitespace" | "comment", _) => Vec::new(),
        ("stylesheet" | "qualified-rule", _) => vec![shape(word, below())],
        ("at-rule", _) => vec![shape(word, first("at-keyword-token"))],
        ("function", _) => vec![shape(word, first("function-token"))],
        ("block", "") => first("{-token"),
        ("block", opener) => vec![shape("simple-block", first(&format!("{opener}-token")))],
        ("declaration", name) => {
            let mut children = vec![
                shape("ident-token", Vec::new()),
                shape("colon-token", Vec::new()),
            ];
            children.extend(below());
            if name.ends_with(" important") {
                children.push(shape("important", Vec::new()));
            }
            vec![shape(word, children)]
        }
        ("error", "bad-string" | "bad-url") => {
            vec![shape(&format!("{argument}-token"), Vec::new())]
        }
        ("error", _) => vec![shape("error", Vec::new())],
        ("literal", value) => literal(value),
        (other, _) => vec![shape(&format!("{other}-token"), Vec::new())],
    }
}

/// The tree tinycss2 printed, read back in the standard's names. Nothing when the text is no print.
pub(in super::super) fn dumped(print: &str) -> Option<Shape> {
    let mut open: Vec<(usize, Line)> = Vec::new();
    let mut roots: Vec<Line> = Vec::new();
    for text in print.lines() {
        let depth = (text.len() - text.trim_start().len()) / 2;
        while open.last().is_some_and(|(each, _)| *each >= depth) {
            let (_, done) = open.pop()?;
            match open.last_mut() {
                Some((_, parent)) => parent.below.push(done),
                None => roots.push(done),
            }
        }
        open.push((
            depth,
            Line {
                head: text.trim_start().to_owned(),
                below: Vec::new(),
            },
        ));
    }
    while let Some((_, done)) = open.pop() {
        match open.last_mut() {
            Some((_, parent)) => parent.below.push(done),
            None => roots.push(done),
        }
    }
    let mut root = roots.pop()?;
    if !roots.is_empty() {
        return None;
    }
    // The library skips an `@charset` that is the first rule, and the comparison leaves it out of the standard's tree too.
    if let Some(first) = root
        .below
        .iter()
        .position(|each| !each.head.starts_with("whitespace") && !each.head.starts_with("comment"))
        && root.below[first]
            .head
            .eq_ignore_ascii_case("at-rule charset")
    {
        drop(root.below.remove(first));
    }
    standard_of(&root).pop()
}

/// The text of the element at `at`.
pub(in super::super) fn text_at<'a>(elements: &Elements, text: &'a str, at: Position) -> &'a str {
    let (from, to) = (start_at(elements, at).0, end_at(elements, at).0);
    text.get(usize::try_from(from).unwrap_or(0)..usize::try_from(to).unwrap_or(0))
        .unwrap_or_default()
}

/// Drops the `@charset` rule that is the first rule, from its at-keyword to its `;` or the `}` of its block: the library skips it
/// without a node, so its tokens lie in the stylesheet.
pub(in super::super) fn skip_charset(elements: &Elements, text: &str, kids: &mut Vec<Position>) {
    let of = |position: &Position| kind_of(kind_at(elements, *position));
    let mut words = kids.iter().copied().filter(|each| {
        !matches!(
            of(each),
            Kind::Token(Token::Whitespace) | Kind::Extra(Extra::Comment | Extra::ByteOrderMark)
        )
    });
    if let Some(first) = words.next()
        && of(&first) == Kind::Token(Token::AtKeyword)
        && text_at(elements, text, first).eq_ignore_ascii_case("@charset")
        && let Some(last) =
            words.find(|each| matches!(of(each), Kind::Token(Token::Semicolon | Token::RightCurly)))
    {
        kids.retain(|each| *each < first || *each > last);
    }
}

/// Our tree at `at` in the standard's terms: what the print has. The print drops white space, comments and the byte order mark, a
/// rule's `}` and `;` and a block's or function's closing token, a stylesheet's `<!--` and `-->`; it has no rule for a qualified rule
/// with no block (an error), and `!important` is a flag of a declaration.
pub(in super::super) fn project(
    elements: &Elements,
    text: &str,
    at: Position,
    out: &mut Vec<Shape>,
) {
    let kind = kind_of(kind_at(elements, at));
    let node = match kind {
        Kind::Token(Token::Whitespace) | Kind::Extra(Extra::Comment | Extra::ByteOrderMark) => {
            return;
        }
        Kind::Token(Token::RightParenthesis | Token::RightSquare | Token::RightCurly) => {
            return out.push(shape("error", Vec::new()));
        }
        Kind::Node(node) => node,
        Kind::Token(_) | Kind::Extra(_) => {
            return out.push(shape(kind_name(kind).0, Vec::new()));
        }
    };
    let mut kids: Vec<Position> = children_at(elements, at).collect();
    let of = |position: &Position| kind_of(kind_at(elements, *position));
    let closer = match (node, kids.first().map(of)) {
        (Node::Function, _) => Some(Token::RightParenthesis),
        (Node::SimpleBlock, Some(Kind::Token(opener))) => mirror(opener),
        _ => None,
    };
    let mut important = false;
    match node {
        Node::Stylesheet => {
            skip_charset(elements, text, &mut kids);
            kids.retain(|each| {
                !matches!(
                    of(each),
                    Kind::Token(Token::Cdo | Token::Cdc | Token::Semicolon)
                )
            });
        }
        Node::QualifiedRule | Node::AtRule => {
            if node == Node::QualifiedRule
                && !kids
                    .iter()
                    .any(|each| of(each) == Kind::Token(Token::LeftCurly))
            {
                return out.push(shape("error", Vec::new()));
            }
            kids.retain(|each| {
                !matches!(of(each), Kind::Token(Token::Semicolon | Token::RightCurly))
            });
        }
        Node::Declaration => {
            let words: Vec<&Position> = kids
                .iter()
                .filter(|each| {
                    !matches!(
                        of(each),
                        Kind::Token(Token::Whitespace) | Kind::Extra(Extra::Comment)
                    )
                })
                .collect();
            if let [.., bang, word] = words.as_slice()
                && of(bang) == Kind::Token(Token::Delim)
                && text_at(elements, text, **bang) == "!"
                && of(word) == Kind::Token(Token::Ident)
                && text_at(elements, text, **word).eq_ignore_ascii_case("important")
            {
                let from = **bang;
                kids.retain(|each| *each < from);
                important = true;
            }
        }
        Node::Function | Node::SimpleBlock => {
            if let (Some(last), Some(closer)) = (kids.last(), closer)
                && of(last) == Kind::Token(closer)
            {
                let _: Option<Position> = kids.pop();
            }
        }
    }
    let mut children = Vec::new();
    for each in kids {
        project(elements, text, each, &mut children);
    }
    if important {
        children.push(shape("important", Vec::new()));
    }
    out.push(shape(kind_name(kind).0, children));
}

/// Our tree of `text` in the standard's terms.
pub(in super::super) fn projected(text: &str) -> Option<Shape> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes()));
    SyntaxCssparserAdapter.read(
        SourceText(text.as_bytes()),
        SourcePath("a.css"),
        &mut builder,
    );
    let mut out = Vec::new();
    project(&elements, text, Position(0), &mut out);
    out.pop()
}

/// The first place the two trees differ, with the path to it; nothing when they are the same.
pub(in super::super) fn differences(ours: &Shape, theirs: &Shape, path: &str) -> Option<String> {
    if ours.label != theirs.label {
        return Some(format!(
            "{path}: we have `{}`, the standard has `{}`",
            ours.label, theirs.label
        ));
    }
    let path = format!("{path}/{}", ours.label);
    ours.children
        .iter()
        .zip(&theirs.children)
        .enumerate()
        .find_map(|(place, (mine, yours))| differences(mine, yours, &format!("{path}[{place}]")))
        .or_else(|| {
            (ours.children.len() != theirs.children.len()).then(|| {
                format!(
                    "{path}: we have {} parts, the standard has {}",
                    ours.children.len(),
                    theirs.children.len()
                )
            })
        })
}
