//! Python expression operand facts at their syntax nodes.
use super::finish::{self, Run, Table, Ty};
use ryo::languages::Position;
use ryo_python::fixtures::columns::ColumnFile;
pub(super) const CALL_SITE: &str = "call-site";
pub(super) const BINARY_OPERANDS: &str = "binary-operands";

pub(super) fn declare(run: &mut Run) {
    use Ty::{File, Node, Num, Sym};
    run.declare(
        CALL_SITE,
        &[
            ("id", Node),
            ("function", Node),
            ("name", Sym),
            ("attribute", Sym),
            ("receiver", Node),
            ("positional", Num),
            ("keywords", Num),
            ("first", Node),
            ("string-annotation", Num),
            ("file", File),
        ],
        finish::Supplier::Provider,
    );
    run.declare(
        BINARY_OPERANDS,
        &[
            ("id", Node),
            ("left", Node),
            ("operator", Sym),
            ("right", Node),
            ("file", File),
        ],
        finish::Supplier::Provider,
    );
}

pub(super) fn fill(
    run: &mut Run,
    file: &ColumnFile<'_, '_>,
    number: u32,
    relation: &str,
) -> Result<Table, String> {
    let (_, names, types) = run
        .catalog(relation)
        .ok_or_else(|| format!("missing {relation}"))?;
    let mut table = Table::with_rows(names, types, 0);
    let file_id = u64::from(number);
    let mut write = |row: &[u64]| {
        for (column, value) in table.cols.iter_mut().zip(row) {
            column.push(*value);
        }
        table.n += 1;
    };
    if relation == CALL_SITE {
        let optional = |node: Option<Position>| node.map_or(finish::NULL, |id| u64::from(id.0));
        file.visit_call_sites(|call| {
            write(&[
                u64::from(call.call.0),
                u64::from(call.function.0),
                u64::from(run.sym(call.name.as_bytes())),
                u64::from(run.sym(call.attribute.as_bytes())),
                optional(call.receiver),
                call.positional as u64,
                call.keywords as u64,
                optional(call.first),
                u64::from(call.string_annotation),
                file_id,
            ])
        });
    } else {
        file.visit_binary_operands(|id, left, op, right| {
            write(&[
                u64::from(id.0),
                u64::from(left.0),
                u64::from(run.sym(op.as_bytes())),
                u64::from(right.0),
                file_id,
            ])
        });
    }
    Ok(table)
}
