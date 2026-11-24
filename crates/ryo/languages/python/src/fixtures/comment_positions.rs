//! Physical line and indentation of a comment, in the reader's source coordinates.
use crate::domain::{
    syntax::{service::kind_bits, types::Kind},
    tokens::types::Token,
};
use ryo::languages as l;

pub(crate) struct Comments;
const SCHEMA: l::FileRelation = l::FileRelation {
    name: "comment-position",
    columns: &[
        ("id", l::CellKind::Node),
        ("file", l::CellKind::File),
        ("span", l::CellKind::Span),
        ("line", l::CellKind::Number),
        ("own-line", l::CellKind::Number),
    ],
};

/// Byte offsets of physical line starts. A CRLF is a single terminator.
pub fn starts(src: &[u8]) -> Vec<u32> {
    let mut offsets = vec![0];
    for i in memchr::memchr2_iter(b'\r', b'\n', src) {
        if src[i] == b'\n' || src.get(i + 1) != Some(&b'\n') {
            offsets.push((i + 1).try_into().expect("Python parser offset"));
        }
    }
    offsets
}

/// The first physical line may start with a BOM before its lexical whitespace.
pub fn position(src: &[u8], starts: &[u32], offset: u32) -> (u64, u64) {
    let line = starts.partition_point(|at| *at <= offset);
    let prefix = &src[starts[line - 1] as usize..offset as usize];
    let prefix = if line == 1 {
        prefix.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(prefix)
    } else {
        prefix
    };
    (
        line as u64,
        u64::from(prefix.iter().all(|b| matches!(b, b' ' | b'\t' | 12))),
    )
}

const BREAK_SCHEMA: l::FileRelation = l::FileRelation {
    name: "comment-line-breaks",
    columns: &[
        ("id", l::CellKind::Node),
        ("file", l::CellKind::File),
        ("previous-end", l::CellKind::Number),
        ("logical-end", l::CellKind::Number),
        ("following-end", l::CellKind::Number),
        ("has-logical", l::CellKind::Number),
        ("has-following", l::CellKind::Number),
    ],
};
const MULTILINE_SCHEMA: l::FileRelation = l::FileRelation {
    name: "multiline-lexeme",
    columns: &[
        ("id", l::CellKind::Node),
        ("file", l::CellKind::File),
        ("line", l::CellKind::Number),
        ("endline", l::CellKind::Number),
        ("form", l::CellKind::Symbol),
        ("component-start", l::CellKind::Number),
        ("component-end", l::CellKind::Number),
    ],
};
/// Complete lexical relation schemas in query column order.
pub const SCHEMAS: &[l::FileRelation] = &[SCHEMA, BREAK_SCHEMA, MULTILINE_SCHEMA];

/// Lend physical lexical facts from the numbered tree. Symbols are bound to the consuming dictionary.
pub fn visit(
    relation: &str,
    tree: &l::NodeTable,
    src: &[u8],
    file: l::FileId,
    mut word: impl FnMut(&'static str) -> u32,
    mut write: impl FnMut(&[u64]),
) -> Result<(), String> {
    if !SCHEMAS.iter().any(|schema| schema.name == relation) {
        return Err(format!("no lexical table {relation}"));
    }
    let lines = std::cell::OnceCell::new();
    let starts = || lines.get_or_init(|| self::starts(src));
    let file_id = u64::from(file.0);
    if relation == SCHEMA.name {
        // A comment is on its own physical line if the line prefix contains only lexical whitespace or the initial BOM.
        for &id in l::node_table::ids_of_kind(tree, kind_bits(Kind::Token(Token::Comment))) {
            let span = l::node_table::span_of(tree, id);
            let (line, standalone) = position(src, starts(), span.start.0);
            write(&[
                u64::from(id.0),
                file_id,
                u64::from(span.start.0) << 32 | u64::from(span.end.0),
                line,
                standalone,
            ]);
        }
    } else if relation == BREAK_SCHEMA.name {
        let offset = |at: Option<u32>| at.map_or(u64::MAX, u64::from);
        super::columns::lexical_lines::breaks(tree, |id, last, logical, following| {
            write(&[
                id.0.into(),
                file_id,
                last.into(),
                offset(logical),
                offset(following),
                logical.is_some().into(),
                following.is_some().into(),
            ])
        });
    } else {
        // Python's multiline lexical unit reaches from its first physical line to the line containing its end offset.
        let mut extents = Vec::new();
        super::columns::lexical_lines::multiline(tree, src, |id, form| {
            let span = l::node_table::span_of(tree, id);
            let lines = starts();
            let first = lines.partition_point(|at| *at <= span.start.0);
            let last = lines.partition_point(|at| *at <= span.end.0);
            if first < last {
                extents.push((first, last, id.0, word(form)));
            }
        });
        // Overlapping inclusive physical-line extents form a connected component, regardless of lexical form.
        extents.sort_unstable();
        let mut component = extents.as_slice();
        while let Some(&(first, mut last, _, _)) = component.first() {
            let mut count = 1;
            while count < component.len() && component[count].0 <= last {
                last = last.max(component[count].1);
                count += 1;
            }
            for &(a, b, id, form) in &component[..count] {
                write(&[
                    id.into(),
                    file_id,
                    a as u64,
                    b as u64,
                    form.into(),
                    first as u64,
                    last as u64,
                ]);
            }
            component = &component[count..];
        }
    }

    Ok(())
}
impl l::FileRelations for Comments {
    fn declarations(&self) -> &'static [l::FileRelation] {
        SCHEMAS
    }
    fn read(
        &self,
        relations: &[&l::FileRelation],
        request: l::ParseRequest<'_>,
        _: &l::Elements,
        tree: &l::NodeTable,
        symbols: &mut l::Symbols,
        _: Option<&dyn l::FileProject>,
    ) -> Vec<Result<Vec<l::FactColumn>, l::ParseError>> {
        let bytes = request.bytes;
        let file = request.file;
        relations.iter().map(|&schema| {
            columns(schema, file, tree, bytes, symbols)
        }).collect()
    }
}

pub(crate) fn columns(
    schema: &l::FileRelation,
    file: l::FileId,
    tree: &l::NodeTable,
    bytes: l::ByteSlice<'_>,
    symbols: &mut l::Symbols,
) -> Result<Vec<l::FactColumn>, l::ParseError> {
    if !SCHEMAS.contains(schema) {
        return Err(l::ParseError::Unsupported);
    }
    let mut cols = vec![l::FactColumn::default(); schema.columns.len()];
    visit(
        schema.name,
        tree,
        bytes.0,
        file,
        |s| l::intern_derived(symbols, bytes, l::NameText(s)).0,
        |row| {
            for ((col, (_, ty)), &raw) in cols.iter_mut().zip(schema.columns).zip(row) {
                col.0.push(if *ty == l::CellKind::Node {
                    (u64::from(file.0) << 32) | raw
                } else {
                    raw
                });
            }
        },
    )
    .map_err(|reason| l::ParseError::Library {
        reason: l::ToolMessage(reason.into()),
    })?;
    Ok(cols)
}
