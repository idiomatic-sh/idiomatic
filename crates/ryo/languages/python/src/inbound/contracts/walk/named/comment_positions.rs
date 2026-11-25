//! Physical line and indentation of a comment, in the reader's source coordinates.
use crate::domain::{
    syntax::{service::kind_bits, types::Kind},
    tokens::types::Token,
};
use ryo::languages as l;

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
pub(crate) fn starts(src: &[u8]) -> Vec<u32> {
    let mut offsets = vec![0];
    for i in memchr::memchr2_iter(b'\r', b'\n', src) {
        if src[i] == b'\n' || src.get(i + 1) != Some(&b'\n') {
            offsets.push((i + 1).try_into().expect("Python parser offset"));
        }
    }
    offsets
}

/// The first physical line may start with a BOM before its lexical whitespace.
pub(crate) fn position(src: &[u8], starts: &[u32], offset: u32) -> (u64, u64) {
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

/// Complete physical comment positions in query column order.
pub(crate) const SCHEMAS: &[l::FileRelation] = &[SCHEMA];

pub(crate) fn columns(
    schema: &l::FileRelation,
    file: l::FileId,
    tree: &l::NodeTable,
    bytes: l::ByteSlice<'_>,
) -> Result<Vec<l::FactColumn>, l::ParseError> {
    if *schema != SCHEMA {
        return Err(l::ParseError::Unsupported);
    }
    let mut cols = vec![l::FactColumn::default(); schema.columns.len()];
    let comments = l::node_table::ids_of_kind(tree, kind_bits(Kind::Token(Token::Comment)));
    if comments.is_empty() {
        return Ok(cols);
    }
    let starts = starts(bytes.0);
    let file_id = u64::from(file.0);
    for &id in comments {
        let span = l::node_table::span_of(tree, id);
        let (line, own_line) = position(bytes.0, &starts, span.start.0);
        let row = [
            file_id << 32 | u64::from(id.0),
            file_id,
            u64::from(span.start.0) << 32 | u64::from(span.end.0),
            line,
            own_line,
        ];
        for (column, value) in cols.iter_mut().zip(row) {
            column.0.push(value);
        }
    }
    Ok(cols)
}
