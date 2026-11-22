//! Embed the expressions of the strings the language reads as type expressions: parse each the way a string annotation is read,
//! write its expression as an embedded segment of the table with the walker of the file's own tree, and ask again for the
//! strings of those expressions.
use super::lexical::offset;
use super::types::{
    ByteIndex, Depth, FutureAnnotations, ParsedString, StringParse, TextAround, TokenIndex, Walker,
};
use crate::domain::model::lexical::types::EmbeddedSite;
use crate::domain::node::service::table_id;
use crate::domain::node::types::Walked;
use crate::domain::node::types::WalkedTree;
use ruff_python_ast::AnyNodeRef;
use ruff_python_ast::visitor::source_order::SourceOrderVisitor as _;
use ruff_python_parser::parse_expression;
use ruff_python_parser::typing::parse_type_annotation;
use ruff_text_size::{Ranged as _, TextRange, TextSize};
use ryo::languages::{self, EmbeddedMapping, NameText, Offset, Position, node_table};

/// The text the offsets of the nodes of the string at `host` are of, and how its bytes map to the file's: the file's own text
/// for a string of the file's own tree and of an expression read in place, the decoded value of the string that holds it
/// otherwise.
fn text_around<'p>(
    tree: &WalkedTree<'p>,
    texts: &[NameText<'p>],
    host: Position,
) -> TextAround<'p> {
    let after = tree
        .segments
        .partition_point(|segment| segment.first <= host);
    let around = after.checked_sub(1).and_then(|within| {
        Some(TextAround {
            text: texts.get(within).copied()?,
            mapping: tree.segments.get(within)?.mapping,
        })
    });
    match around {
        Some(TextAround {
            text,
            mapping: EmbeddedMapping::WholeHost,
        }) => TextAround {
            text,
            mapping: EmbeddedMapping::WholeHost,
        },
        Some(TextAround {
            mapping: EmbeddedMapping::Exact(_),
            ..
        })
        | None => TextAround {
            text: tree.text,
            mapping: EmbeddedMapping::Exact(Offset(0)),
        },
    }
}

/// Parses the string at `host` as a type expression, the way the language reads a string annotation: in place when the string
/// is written plainly, from its decoded value when it has escapes or is written in parts. None when it is no expression.
fn parsed_string<'p>(
    tree: &WalkedTree<'p>,
    texts: &[NameText<'p>],
    host: Position,
) -> Option<ParsedString<'p>> {
    let AnyNodeRef::ExprStringLiteral(literal) = tree.nodes.get(usize::from(host)).copied()? else {
        return None;
    };
    let TextAround {
        text,
        mapping: inherited,
    } = text_around(tree, texts, host);
    // A string that is no expression is embedded as none: the language reads it as the string it is.
    let Ok(annotation) = parse_type_annotation(literal, text.0) else {
        return None;
    };
    if annotation.kind().is_simple() {
        let (start, end) = (
            Offset(annotation.expression().start().to_u32()),
            Offset(annotation.expression().end().to_u32()),
        );
        let mapping = match inherited {
            EmbeddedMapping::Exact(_) => EmbeddedMapping::Exact(start),
            EmbeddedMapping::WholeHost => EmbeddedMapping::WholeHost,
        };
        return Some(ParsedString {
            host,
            parsed: StringParse::InPlace(annotation),
            text,
            start,
            end,
            mapping,
        });
    }
    let decoded = literal.value.to_str();
    let Ok(parsed) = parse_expression(decoded) else {
        return None;
    };
    Some(ParsedString {
        host,
        parsed: StringParse::Decoded(parsed),
        text: NameText(decoded),
        start: Offset(0),
        end: offset(ByteIndex(decoded.len())),
        mapping: EmbeddedMapping::WholeHost,
    })
}

/// Writes the expression of one parsed string as an embedded segment of the table, its nodes after the tree's.
fn write_segment<'q>(
    builder: &mut languages::NodeTableBuilder<'_>,
    tree: &mut WalkedTree<'q>,
    texts: &mut Vec<NameText<'q>>,
    string: &'q ParsedString<'q>,
) {
    let parsed = match &string.parsed {
        StringParse::InPlace(annotation) => annotation.parsed(),
        StringParse::Decoded(parsed) => parsed,
    };
    let (from, to) = (
        usize::try_from(string.start.0).unwrap_or(usize::MAX),
        usize::try_from(string.end.0).unwrap_or(usize::MAX),
    );
    let written = NameText(string.text.0.get(from..to).unwrap_or_default());
    let root = node_table::embed(
        builder,
        table_id(&tree.segments, string.host),
        languages::EmbeddedText {
            text: written,
            mapping: string.mapping,
        },
    );
    let first = Position(u32::try_from(tree.nodes.len()).unwrap_or(u32::MAX));
    // The parentheses and the trivia around the expression are outside it: only the tokens of its range are its own.
    let tokens = parsed.tokens().in_range(TextRange::new(
        TextSize::new(string.start.0),
        TextSize::new(string.end.0),
    ));
    let mut walker = Walker {
        builder,
        tokens,
        next: TokenIndex(0),
        written: Offset(0),
        ends: Vec::new(),
        strings: Vec::new(),
        parsed_tokens: parsed.tokens(),
        refused: Vec::new(),
        future_annotations: FutureAnnotations(false),
        annotations_deferred: false,
        length: Offset(string.end.0.saturating_sub(string.start.0)),
        nodes: std::mem::take(&mut tree.nodes),
        parents: std::mem::take(&mut tree.parents),
        subtree_ends: std::mem::take(&mut tree.ends),
        open: Vec::new(),
        fields: Vec::new(),
        origin: string.start,
        host: Some(string.host),
        // Every string of a type expression stands where the language writes a type.
        annotating: Depth(1),
        literals: std::mem::take(&mut tree.strings),
        imports: std::mem::take(&mut tree.imports),
    };
    walker.visit_expr(parsed.expr());
    tree.nodes = walker.nodes;
    tree.parents = walker.parents;
    tree.ends = walker.subtree_ends;
    tree.strings = walker.literals;
    tree.imports = walker.imports;
    tree.segments.push(EmbeddedSite {
        host: string.host,
        first,
        root: root.0,
        mapping: string.mapping,
    });
    texts.push(string.text);
}

/// Asks `read` which of the strings the walk met since it last asked are type expressions, embeds the expression of each,
/// and asks again for the strings of those expressions; lends the tree once none is one. The parses of one round live in this
/// call, so the tree a later round lends still reads them.
pub(super) fn embedded<'p>(
    builder: &mut languages::NodeTableBuilder<'_>,
    mut tree: WalkedTree<'p>,
    texts: Vec<NameText<'p>>,
    read: &mut dyn FnMut(Walked<'_, '_, '_>) -> Vec<Position>,
) {
    let hosts = read(Walked::Strings(&tree));
    tree.strings.clear();
    let parsed: Vec<ParsedString<'p>> = hosts
        .iter()
        .filter_map(|host| parsed_string(&tree, &texts, *host))
        .collect();
    if parsed.is_empty() {
        let _nothing = read(Walked::Whole(builder, tree));
        return;
    }
    let mut tree: WalkedTree<'_> = tree;
    let mut texts: Vec<NameText<'_>> = texts;
    for string in &parsed {
        write_segment(builder, &mut tree, &mut texts, string);
    }
    embedded(builder, tree, texts, read);
}
