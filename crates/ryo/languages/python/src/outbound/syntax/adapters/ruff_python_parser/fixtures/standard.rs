//! The standard's tree and ours set side by side: what `CPython`'s `ast.dump` printed for a file, read back as a tree of node names,
//! and our tree of the same file projected onto the standard, so that the two can be compared node for node, each in the field of its
//! parent that holds it.

use crate::domain::extras::types::Extra;
use crate::domain::fields::service::field_of;
use crate::domain::labels::service::field_name;
use crate::domain::labels::types::Field;
use crate::domain::nodes::service::category;
use crate::domain::nodes::types::{Category, Node};
use crate::domain::syntax::service::{kind_name, kind_of};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::PythonSourceKind;
use crate::outbound::syntax::types::SourceText;
use crate::settings::PythonParserSettings;
use ryo::languages::{
    ByteSlice, Elements, FileId, Position, Symbols, TOKEN, Tree, children as children_at, end_at,
    kind_at, start_at, table_builder,
};
use strum::IntoEnumIterator as _;

use super::super::adapter::SyntaxRuffPythonParserAdapter;

/// A node of the standard's tree: the field of its parent that holds it and its name, in kebab case as the roles and the node kinds
/// are named, and the nodes under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Shape {
    /// The field of its parent that holds it; empty for the root.
    pub field: String,
    /// The name.
    pub name: String,
    /// The nodes under it, in the order met.
    pub children: Vec<Shape>,
}

/// A node of the standard's tree named `name` in the field `field`, with `children` under it.
fn shape(field: &str, name: &str, children: Vec<Shape>) -> Shape {
    Shape {
        field: field.to_owned(),
        name: name.to_owned(),
        children,
    }
}

/// Where the dump is read, inside a constructor's arguments or a list: the field the next value is given for, and the field of a
/// list's elements.
#[derive(Default)]
struct Frame {
    /// The field named by the last `name=`, until a value ends it.
    pending: Option<String>,
    /// The field of the list the frame is, for each of its elements.
    list: Option<String>,
}

/// A name of the standard in kebab case: `FunctionDef` is `function-def`, `match_case` is `match-case`.
fn kebab(name: &str) -> String {
    name.char_indices()
        .flat_map(|(place, letter)| {
            let upper = letter.is_ascii_uppercase() && place > 0;
            upper.then_some('-').into_iter().chain([if letter == '_' {
                '-'
            } else {
                letter.to_ascii_lowercase()
            }])
        })
        .collect()
}

/// The name a node is compared by: one kind for what the language does not split, `async` and `except*` read from a keyword.
fn unsplit(name: String) -> String {
    match (name.strip_prefix("async-"), name.as_str()) {
        (Some(rest), _) => rest.to_owned(),
        (None, "try-star") => "try".to_owned(),
        (None, _) => name,
    }
}

/// Whether a name of the standard is one no node of a tree stands for: an operator, a context, a type comment.
fn is_no_node(name: &str) -> bool {
    name == "type-ignore"
        || Node::iter().any(|node| {
            kind_name(Kind::Node(node)).0 == name && category(node) == Category::Operator
        })
}

/// The tree `CPython`'s `ast.dump` printed, read back: each constructor called is a node in the field it is the value of (an
/// element of a list in the list's field), the nodes in its arguments under it, the values (names, strings, numbers) dropped.
/// Nothing when the text is no dump.
#[must_use]
pub(crate) fn dumped(dump: &str) -> Option<Shape> {
    let mut open: Vec<Shape> = vec![shape("", "", Vec::new())];
    let mut frames: Vec<Frame> = vec![Frame::default()];
    let bytes = dump.as_bytes();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b'\'' | b'"' => {
                at += 1;
                while let Some(&inner) = bytes.get(at) {
                    if inner == byte {
                        break;
                    }
                    at += if inner == b'\\' { 2 } else { 1 };
                }
                at += 1;
            }
            b'[' => {
                let field = frames.last_mut()?.pending.take();
                frames.push(Frame {
                    pending: None,
                    list: field,
                });
                at += 1;
            }
            b']' => {
                drop(frames.pop()?);
                at += 1;
            }
            b',' => {
                frames.last_mut()?.pending = None;
                at += 1;
            }
            b')' => {
                drop(frames.pop()?);
                let done = open.pop()?;
                let done = shape(&done.field, &unsplit(done.name), done.children);
                if !is_no_node(&done.name) {
                    open.last_mut()?.children.push(done);
                }
                at += 1;
            }
            letter if letter.is_ascii_alphabetic() || letter == b'_' => {
                let start = at;
                while bytes
                    .get(at)
                    .is_some_and(|each| each.is_ascii_alphanumeric() || *each == b'_')
                {
                    at += 1;
                }
                let word = dump.get(start..at)?;
                match bytes.get(at) {
                    Some(b'(') => {
                        let frame = frames.last_mut()?;
                        let field = frame
                            .pending
                            .take()
                            .or_else(|| frame.list.clone())
                            .unwrap_or_default();
                        open.push(shape(&kebab(&field), &kebab(word), Vec::new()));
                        frames.push(Frame::default());
                        at += 1;
                    }
                    Some(b'=') => {
                        frames.last_mut()?.pending = Some(word.to_owned());
                        at += 1;
                    }
                    Some(_) | None => {}
                }
            }
            _ => at += 1,
        }
    }
    let mut root = open.pop()?;
    if !open.is_empty() || root.children.len() != 1 {
        return None;
    }
    root.children.pop()
}

/// The name of the role of the element at `at` under its parent: empty when it has none.
fn role(tree: &Tree<'_>, at: Position) -> &'static str {
    let field = field_of(tree, at);
    if field == Field::None {
        ""
    } else {
        field_name(field).0
    }
}

/// Our tree at `at` projected onto the standard: the nodes it is, tokens dropped, each extra replaced by what it holds, an `elif`
/// clause nested as an `If` in the `orelse` of the one before, a string part of an f-string as the `Constant` it is, adjacent
/// constants of an f-string joined as `CPython` joins them, the text of a self-documenting interpolation as
/// the `Constant` `CPython` writes before it, and a lambda with no parameters given its empty `arguments`. Each node is in its own
/// role, or, under an extra, in the extra's when it has none of its own: `inherited` is that of the extra above it.
fn project(tree: &Tree<'_>, text: &str, at: Position, inherited: &str, out: &mut Vec<Shape>) {
    let elements = tree.elements;
    let kind = kind_of(kind_at(elements, at));
    let own = role(tree, at);
    let field = if own.is_empty() { inherited } else { own };
    match kind {
        Kind::Token(_) | Kind::Extra(Extra::Whitespace | Extra::ElifElseClause) => {}
        Kind::Extra(Extra::StringPart) => {
            let parent = elements
                .parent
                .get(place(at))
                .map(|above| kind_of(kind_at(elements, *above)));
            if matches!(
                parent,
                Some(Kind::Node(Node::JoinedStr | Node::TemplateStr))
            ) {
                out.push(shape(field, "constant", Vec::new()));
            }
        }
        Kind::Extra(_) => {
            for each in children_at(elements, at) {
                project(tree, text, each, field, out);
            }
        }
        Kind::Node(node) => {
            let mut children = Vec::new();
            let mut clauses = Vec::new();
            for each in children_at(elements, at) {
                if kind_of(kind_at(elements, each)) == Kind::Extra(Extra::ElifElseClause) {
                    clauses.push(each);
                } else {
                    project(tree, text, each, "", &mut children);
                }
            }
            let mut orelse: Vec<Shape> = Vec::new();
            for clause in clauses.into_iter().rev() {
                let mut body = Vec::new();
                children_at(elements, clause)
                    .for_each(|each| project(tree, text, each, "", &mut body));
                let elif = children_at(elements, clause)
                    .find(|each| {
                        elements
                            .flags
                            .get(place(*each))
                            .is_some_and(|flags| flags.0 & TOKEN.0 != 0)
                            && kind_of(kind_at(elements, *each)) != Kind::Extra(Extra::Whitespace)
                    })
                    .is_some_and(|keyword| {
                        let (from, to) =
                            (start_at(elements, keyword).0, end_at(elements, keyword).0);
                        text.get(
                            usize::try_from(from).unwrap_or(0)..usize::try_from(to).unwrap_or(0),
                        ) == Some("elif")
                    });
                if elif {
                    body.append(&mut orelse);
                    orelse = vec![shape(role(tree, clause), "if", body)];
                } else {
                    // An `else` clause's statements are the `orelse` of the `if` it closes.
                    for each in &mut body {
                        role(tree, clause).clone_into(&mut each.field);
                    }
                    orelse = body;
                }
            }
            children.append(&mut orelse);
            if node == Node::Lambda && !children.iter().any(|each| each.name == "arguments") {
                children.push(shape("args", "arguments", Vec::new()));
            }
            if matches!(node, Node::JoinedStr | Node::TemplateStr) {
                children.dedup_by(|later, earlier| {
                    later.name == "constant"
                        && earlier.name == "constant"
                        && later.children.is_empty()
                        && earlier.children.is_empty()
                });
            }
            // CPython writes the text of a self-documenting interpolation, `{x=}`, as a `Constant` before it.
            if is_self_documenting(elements, at) {
                out.push(shape(field, "constant", Vec::new()));
            }
            out.push(Shape {
                field: field.to_owned(),
                // The reference AST stores Async* and TryStar as distinct constructors. Our lossless tree retains Async* kinds,
                // but its syntax-token keyword is not an AST child, so normalize these names to the documented split boundary.
                name: unsplit(kind_name(kind).0.to_owned()),
                children,
            });
        }
    }
}

/// Whether the element at `at` is an interpolation that documents itself, `{x=}`.
fn is_self_documenting(elements: &Elements, at: Position) -> bool {
    matches!(
        kind_of(kind_at(elements, at)),
        Kind::Node(Node::FormattedValue | Node::Interpolation)
    ) && children_at(elements, at)
        .any(|each| kind_of(kind_at(elements, each)) == Kind::Token(Token::Equal))
}

/// Our tree of `text`, projected onto the standard.
#[must_use]
pub(crate) fn projected(text: &str) -> Option<Shape> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    SyntaxRuffPythonParserAdapter {
        settings: PythonParserSettings::default(),
        ..Default::default()
    }
    .read(
        SourceText(text.as_bytes()),
        PythonSourceKind::Python,
        &mut table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes())),
    );
    let tree = Tree {
        elements: &elements,
        source: ByteSlice(text.as_bytes()),
        file: FileId(0),
    };
    let mut out = Vec::new();
    project(&tree, text, Position(0), "", &mut out);
    out.pop()
}

/// A shape written so that shapes with the same nodes in the same fields under the same parents are equal, whatever their order:
/// the standard keeps a node's fields apart, the tree keeps them in the order of the file.
#[must_use]
pub(crate) fn canonical(shape: &Shape) -> String {
    let mut children: Vec<String> = shape.children.iter().map(canonical).collect();
    children.sort();
    if shape.field.is_empty() {
        format!("{}({})", shape.name, children.join(" "))
    } else {
        format!("{}={}({})", shape.field, shape.name, children.join(" "))
    }
}

/// What differs between our projected tree and `CPython`'s, the first place found; nothing when they agree.
#[must_use]
pub(crate) fn differences(ours: &Shape, theirs: &Shape, path: &str) -> Option<String> {
    if ours.name != theirs.name {
        return Some(format!(
            "{path}: we have `{}`, CPython has `{}`",
            ours.name, theirs.name
        ));
    }
    let sorted = |shapes: &[Shape]| -> Vec<(String, Shape)> {
        let mut found: Vec<(String, Shape)> = shapes
            .iter()
            .map(|each| (canonical(each), each.clone()))
            .collect();
        found.sort_by(|left, right| left.0.cmp(&right.0));
        found
    };
    let (mine, yours) = (sorted(&ours.children), sorted(&theirs.children));
    // A multiset difference: each child of one side cancels at most one equal child of the other, so a repeated child counts as
    // often as it is written.
    let only = |from: &[(String, Shape)], other: &[(String, Shape)]| -> Vec<(String, Shape)> {
        let mut unmatched: Vec<&(String, Shape)> = other.iter().collect();
        let mut left = Vec::new();
        for each in from {
            match unmatched.iter().position(|one| one.0 == each.0) {
                Some(at) => {
                    let _: &(String, Shape) = unmatched.remove(at);
                }
                None => left.push(each.clone()),
            }
        }
        left
    };
    let (only_mine, only_yours) = (only(&mine, &yours), only(&yours, &mine));
    match (only_mine.first(), only_yours.first()) {
        (None, None) => None,
        (Some(left), Some(right))
            if left.1.name == right.1.name && left.1.field == right.1.field =>
        {
            differences(&left.1, &right.1, &format!("{path}/{}", ours.name))
        }
        _ => Some(format!(
            "{path}/{}: we have {:?}, CPython has {:?}",
            ours.name,
            only_mine.iter().map(|each| &each.0).collect::<Vec<_>>(),
            only_yours.iter().map(|each| &each.0).collect::<Vec<_>>()
        )),
    }
}

/// The place of the element at `at` in the table's columns; past every column when it does not fit in memory.
fn place(at: Position) -> usize {
    usize::try_from(at.0).unwrap_or(usize::MAX)
}
