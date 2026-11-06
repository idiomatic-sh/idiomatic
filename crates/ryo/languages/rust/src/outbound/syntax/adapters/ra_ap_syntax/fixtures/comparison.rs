//! Test support split from adapter_test: read, functions, own_tree and more.

use super::adapter::SyntaxRaApSyntaxAdapter;
use super::unproduced::kind_of_bits;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::EXPANDED_MACROS;
use crate::domain::syntax::service::kind_name;
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::{Expanding, SourceText};
use ryo::languages::{
    ByteSlice, Elements, Position, Symbol, Symbols, TOKEN, end_at, positions, start_at,
    table_builder, text_of,
};

pub(in super::super) fn read(source: &str) -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    SyntaxRaApSyntaxAdapter::new(Expanding(&EXPANDED_MACROS)).read(
        SourceText(source.as_bytes()),
        &mut table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes())),
    );
    (elements, symbols)
}

pub(in super::super) fn functions(source: &'static str) -> String {
    let (elements, symbols) = read(source);
    let functions: Vec<Position> = positions(&elements)
        .filter(|position| kind_of_bits(&elements, *position) == Kind::Node(Node::Fn))
        .collect();
    let first = functions[0];
    let (start, end) = (start_at(&elements, first), end_at(&elements, first));
    let named: Vec<&str> = (0..ryo::languages::contract::name_count(&symbols).0)
        .filter_map(|place| {
            text_of(
                &symbols,
                ByteSlice(source.as_bytes()),
                Symbol(u32::try_from(place).unwrap_or(0)),
            )
            .map(|text| text.0)
        })
        .collect();
    format!(
        "{} {} {} {}",
        functions.len(),
        source
            .get(usize::try_from(start.0).unwrap_or(0)..usize::try_from(end.0).unwrap_or(0))
            .unwrap_or_default(),
        named.contains(&"one") && named.contains(&"two"),
        kind_name(kind_of_bits(&elements, Position(0))).0
    )
}

/// `source` read into a table, with no macro read again: the file's own tree.
pub(in super::super) fn own_tree(source: &str) -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    SyntaxRaApSyntaxAdapter::new(Expanding(&[])).read(
        SourceText(source.as_bytes()),
        &mut table_builder(&mut elements, &mut symbols, ByteSlice(source.as_bytes())),
    );
    elements
}

/// Our tree of `source` printed as the tests of `ra_ap_parser` print theirs: a line for each node, `KIND`, and each token, `KIND "text"`,
/// indented two spaces for each level.
pub(in super::super) fn printed(source: &str) -> String {
    let elements = own_tree(source);
    let mut levels: Vec<usize> = Vec::new();
    let mut out = String::new();
    for at in positions(&elements) {
        let parent = ryo::languages::parent_at(&elements, at);
        let level = if parent == at {
            0
        } else {
            usize::try_from(parent.0)
                .ok()
                .and_then(|place| levels.get(place))
                .map_or(0, |above| above + 1)
        };
        levels.push(level);
        let kind = kind_of_bits(&elements, at);
        let name = kind_name(kind).0.to_uppercase().replace('-', "_");
        out.push_str(&"  ".repeat(level));
        out.push_str(&name);
        // The parser's `ERROR` is a kind the language lists among its tokens, and a node of the tree: it is a leaf when the table says so.
        if ryo::languages::flags_at(&elements, at).0 & TOKEN.0 != 0 {
            let (from, to) = (
                usize::try_from(start_at(&elements, at).0).unwrap_or(0),
                usize::try_from(end_at(&elements, at).0).unwrap_or(0),
            );
            let text = format!(" {:?}", source.get(from..to).unwrap_or_default());
            out.push_str(&text);
        }
        out.push('\n');
    }
    out
}

/// The first line where our tree of `source` is not the pinned `dump`'s tree, whose `error` lines (the parser's errors, which are not
/// the tree) are left out; nothing when they are the same.
pub(in super::super) fn difference(source: &str, dump: &str) -> Option<String> {
    let ours = printed(source);
    let theirs: Vec<&str> = dump
        .lines()
        .take_while(|line| !line.starts_with("error "))
        .collect();
    let mine: Vec<&str> = ours.lines().collect();
    (0..mine.len().max(theirs.len())).find_map(|place| {
        let (have, standard) = (mine.get(place), theirs.get(place));
        (have != standard).then(|| {
            format!(
                "line {}: we have `{}`, the standard has `{}`",
                place + 1,
                have.copied().unwrap_or_default(),
                standard.copied().unwrap_or_default()
            )
        })
    })
}

pub(in super::super) fn broken(source: &'static str) -> String {
    let (elements, _) = read(source);
    positions(&elements)
        .any(|position| kind_of_bits(&elements, position) == Kind::Node(Node::Fn))
        .to_string()
}
