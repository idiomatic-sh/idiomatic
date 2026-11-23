//! Python expression operand facts at their syntax nodes.
use super::finish::{self, Run, Table, Ty};
use ryo::languages::Position;
use ryo_python::fixtures::columns::ColumnFile;
pub(super) const CALL_SITE: &str = "call-site";
pub(super) const BINARY_OPERANDS: &str = "binary-operands";

pub(super) const EXPR_STATEMENTS: &str = "expression-statement-value";
pub(super) const COMPARISONS: &str = "comparison-operands";
pub(super) const DICT_KEYS: &str = "dictionary-display-key";
pub(super) const ASSIGN_NAMES: &str = "assignment-name-predecessor";
pub(super) const NESTED_DELIMITERS: &str = "nested-delimiters";

pub(super) fn declare(run: &mut Run) {
    use Ty::{File, Node, Num, Sym};
    run.declare(
        EXPR_STATEMENTS,
        &[
            ("statement", Node),
            ("id", Node),
            ("kind", Sym),
            ("span", Ty::Span),
            ("file", File),
        ],
        finish::Supplier::Provider,
    );
    run.declare(
        COMPARISONS,
        &[
            ("id", Node),
            ("left", Node),
            ("right", Node),
            ("file", File),
        ],
        finish::Supplier::Provider,
    );
    run.declare(
        DICT_KEYS,
        &[
            ("dictionary", Node),
            ("id", Node),
            ("count", Num),
            ("begin", Num),
            ("file", File),
        ],
        finish::Supplier::Provider,
    );
    run.declare(
        ASSIGN_NAMES,
        &[
            ("assignment", Node),
            ("id", Node),
            ("name", Sym),
            ("previous", Node),
            ("first-direct", Node),
            ("has-direct-pair", Num),
            ("file", File),
        ],
        finish::Supplier::Provider,
    );
    run.declare(
        NESTED_DELIMITERS,
        &[
            ("outer", Node),
            ("inner", Node),
            ("outer-kind", Sym),
            ("inner-kind", Sym),
            ("nonempty", Num),
            ("after-inner-kind", Sym),
            ("file", File),
        ],
        finish::Supplier::Provider,
    );
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
    } else if relation == EXPR_STATEMENTS {
        file.visit_expression_statements(|statement, value| {
            let kind = ryo::languages::node_table::kind_of(file.table, value);
            let span = ryo::languages::node_table::span_of(file.table, value);
            write(&[
                statement.0.into(),
                value.0.into(),
                run.kind_sym[usize::from(kind.0)].into(),
                u64::from(span.start.0) << 32 | u64::from(span.end.0),
                file_id,
            ])
        });
    } else if relation == COMPARISONS {
        file.visit_comparison_operands(|id, left, right| {
            write(&[id.0.into(), left.0.into(), right.0.into(), file_id])
        });
    } else if relation == DICT_KEYS {
        file.visit_dictionary_keys(|dict, key, count| {
            let begin = ryo::languages::node_table::span_of(file.table, key).start.0;
            write(&[
                dict.0.into(),
                key.0.into(),
                count as u64,
                begin.into(),
                file_id,
            ]);
        });
    } else if relation == ASSIGN_NAMES {
        file.visit_repeated_assignment_names(|stmt, id, name, previous, first_direct| {
            write(&[
                stmt.0.into(),
                id.0.into(),
                run.sym(name.as_bytes()).into(),
                previous.0.into(),
                first_direct.map_or(finish::NULL, |at| at.0.into()),
                first_direct.is_some().into(),
                file_id,
            ])
        });
    } else if relation == NESTED_DELIMITERS {
        let kind = |p| {
            u64::from(
                run.kind_sym[usize::from(ryo::languages::node_table::kind_of(file.table, p).0)],
            )
        };
        let mut pairs = Vec::new();
        file.visit_nested_delimiters(|outer, inner| pairs.push((outer, inner)));
        if !pairs.is_empty() {
            // The provider's token order supplies the same matching endpoints as token-link.
            let mut closing: rustc_hash::FxHashMap<u32, Option<Position>> =
                pairs.iter().map(|&(_, inner)| (inner.0, None)).collect();
            let tokens = run.tokens.as_ref().ok_or("missing token order")?;
            tokens.links(file.table, &mut |event| {
                if let finish::TokenEvent::Matching(open, close) = event
                    && let Some(slot) = closing.get_mut(&open)
                {
                    *slot = Some(Position(close));
                }
            });
            for (outer, inner) in pairs {
                let close = closing[&inner.0];
                let nonempty = close
                    .is_some_and(|c| file.next_code_token(inner).is_some_and(|first| first != c));
                let after = close
                    .and_then(|c| file.next_code_token(c))
                    .map_or(finish::NULL, kind);
                write(&[
                    outer.0.into(),
                    inner.0.into(),
                    kind(outer),
                    kind(inner),
                    nonempty.into(),
                    after,
                    file_id,
                ]);
            }
        }
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
