//! Python lexical extents in physical source coordinates.
use super::finish::{Run, Supplier, Table};

use ryo_python::fixtures::columns::ColumnFile;

pub(super) const COMMENT: &str = "comment-position";
pub(super) const BREAKS: &str = "comment-line-breaks";
pub(super) const MULTILINE: &str = "multiline-lexeme";

pub(super) fn declare(run: &mut Run) {
    for schema in ryo_python::fixtures::comment_positions::SCHEMAS {
        let columns: Vec<_> = schema
            .columns
            .iter()
            .map(|&(n, kind)| (n, super::finish::ty_of(kind)))
            .collect();
        run.declare(schema.name, &columns, Supplier::Provider);
    }
}

pub(super) fn fill(
    run: &mut Run,
    file: &ColumnFile<'_, '_>,
    src: &[u8],
    number: u32,
    relation: &str,
) -> Result<Table, String> {
    let (_, names, types) = run
        .catalog(relation)
        .ok_or_else(|| format!("missing {relation}"))?;
    let mut rows = Table::with_rows(names, types, 0);
    let mut write = |row: &[u64]| {
        for (col, value) in rows.cols.iter_mut().zip(row) {
            col.push(*value);
        }
        rows.n += 1;
    };
    ryo_python::fixtures::comment_positions::visit(
        relation,
        file.table,
        src,
        ryo::languages::FileId(number),
        |form| run.sym(form.as_bytes()),
        &mut write,
    )?;
    Ok(rows)
}
