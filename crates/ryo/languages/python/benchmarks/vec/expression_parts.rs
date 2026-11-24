//! Python expression operand facts at their syntax nodes.
use super::finish::{self, Run, Table};
use ryo_python::fixtures::columns::ColumnFile;
pub(super) const CALL_SITE: &str = "call-site";
pub(super) const BINARY_OPERANDS: &str = "binary-operands";

pub(super) const EXPR_STATEMENTS: &str = "expression-statement-value";
pub(super) const BOOLEANS: &str = "boolean-operands";
pub(super) const WITH_HEADER: &str = "with-header";
pub(super) const COMPARISONS: &str = "comparison-operands";
pub(super) const DICT_KEYS: &str = "dictionary-display-key";
pub(super) const ASSIGN_NAMES: &str = "assignment-name-predecessor";
pub(super) const DISPLAY_ELEMENT: &str = "display-element";
pub(super) const POSITIONAL_ARGUMENT: &str = "positional-argument-site";
pub(super) const ATTRIBUTE_PREFIX: &str = "attribute-prefix";
pub(super) const CONSTANT_PREDECESSOR: &str = "dictionary-constant-predecessor";
pub(super) const ANNOTATION_STRINGS: &str = "annotation-string-site";
pub(super) const SUBSCRIPT: &str = "subscript-operands";
pub(super) const SCHEDULE_END: &str = "binding-schedule-end";
pub(super) const NESTED_DELIMITERS: &str = "nested-delimiters";

pub(super) fn declare(run: &mut Run) {
    for schema in ColumnFile::EXPRESSION_SCHEMAS {
        let columns: Vec<_> = schema
            .columns
            .iter()
            .map(|&(name, kind)| (name, finish::ty_of(kind)))
            .collect();
        run.declare(schema.name, &columns, finish::Supplier::Provider);
    }
}

pub(super) fn fill(
    run: &mut Run,
    file: &mut ColumnFile<'_, '_>,
    source: &[u8],
    number: u32,
    relation: &str,
) -> Result<Table, String> {
    let (_, names, types) = run
        .catalog(relation)
        .ok_or_else(|| format!("missing {relation}"))?;
    let mut table = Table::with_rows(names, types, 0);
    file.visit_expression_relation(
        relation,
        source,
        number,
        |word| run.sym(word),
        |row| {
            for (column, value) in table.cols.iter_mut().zip(row) {
                column.push(*value);
            }
            table.n += 1;
        },
    )?;
    Ok(table)
}
