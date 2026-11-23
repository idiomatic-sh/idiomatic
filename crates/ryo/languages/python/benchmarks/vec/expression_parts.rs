//! Python expression operand facts at their syntax nodes.
use super::finish::{self, Run, Table, Ty};
use ryo::languages::Position;
use ryo_python::fixtures::columns::ColumnFile;
pub(super) const CALL_SITE: &str = "call-site";
pub(super) const BITWISE_OR_TREE: &str = "bitwise-or-tree";
pub(super) const BINARY_OPERANDS: &str = "binary-operands";

pub(super) const EXPR_STATEMENTS: &str = "expression-statement-value";
pub(super) const BOOLEANS: &str = "boolean-operands";
pub(super) const WITH_HEADER: &str = "with-header";
pub(super) const COMPARISONS: &str = "comparison-operands";
pub(super) const DICT_KEYS: &str = "dictionary-display-key";
pub(super) const ASSIGNMENT_POSITIONS: &str = "assignment-name-position";
pub(super) const ASSIGN_NAMES: &str = "assignment-name-predecessor";
pub(super) const DISPLAY_ELEMENT: &str = "display-element";
pub(super) const POSITIONAL_ARGUMENT: &str = "positional-argument-site";
pub(super) const ATTRIBUTE_PREFIX: &str = "attribute-prefix";
pub(super) const CONSTANT_PREDECESSOR: &str = "dictionary-constant-predecessor";
pub(super) const ANNOTATION_STRINGS: &str = "annotation-string-site";
pub(super) const SUBSCRIPT: &str = "subscript-operands";
pub(super) const SCHEDULE_END: &str = "binding-schedule-end";
pub(super) const NESTED_DELIMITERS: &str = "nested-delimiters";

pub(super) const DELIMITER_COLUMNS: &[(&str, ryo::languages::CellKind)] = &[
    ("outer", ryo::languages::CellKind::Node),
    ("inner", ryo::languages::CellKind::Node),
    ("outer-kind", ryo::languages::CellKind::Symbol),
    ("inner-kind", ryo::languages::CellKind::Symbol),
    ("nonempty", ryo::languages::CellKind::Number),
    ("after-inner-kind", ryo::languages::CellKind::Symbol),
    ("close", ryo::languages::CellKind::Node),
    ("after", ryo::languages::CellKind::Node),
    ("first-comma", ryo::languages::CellKind::Node),
    ("first-yield", ryo::languages::CellKind::Node),
    ("commas", ryo::languages::CellKind::Number),
    ("yields", ryo::languages::CellKind::Number),
    ("file", ryo::languages::CellKind::File),
];

pub(super) fn declare(run: &mut Run) {
    use Ty::{File, Node, Num, Sym};
    let provider = finish::Supplier::Provider;
    run.declare(
        DISPLAY_ELEMENT,
        &[
            ("owner", Node),
            ("id", Node),
            ("position", Num),
            ("count", Num),
            ("after", Num),
            ("file", File),
        ],
        provider,
    );
    run.declare(
        POSITIONAL_ARGUMENT,
        &[
            ("owner", Node),
            ("id", Node),
            ("position", Num),
            ("file", File),
        ],
        provider,
    );
    run.declare(
        ATTRIBUTE_PREFIX,
        &[
            ("root", Node),
            ("end", Node),
            ("depth", Num),
            ("file", File),
        ],
        provider,
    );
    run.declare(
        CONSTANT_PREDECESSOR,
        &[
            ("dictionary", Node),
            ("id", Node),
            ("previous", Node),
            ("file", File),
        ],
        provider,
    );
    run.declare(
        ANNOTATION_STRINGS,
        &[("id", Node), ("root", Node), ("file", File)],
        provider,
    );
    run.declare(
        SUBSCRIPT,
        &[
            ("id", Node),
            ("value", Node),
            ("slice", Node),
            ("value-name", Sym),
            ("file", File),
        ],
        provider,
    );
    run.declare(SCHEDULE_END, &[("point", Num), ("file", File)], provider);
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
        WITH_HEADER,
        &[
            ("id", Node),
            ("colon", Node),
            ("colon-span", Ty::Span),
            ("file", File),
        ],
        provider,
    );
    for relation in [COMPARISONS, BOOLEANS] {
        run.declare(
            relation,
            &[
                ("id", Node),
                ("left", Node),
                ("right", Node),
                ("operator", Sym),
                ("count", Num),
                ("file", File),
            ],
            provider,
        );
    }
    run.declare(
        DICT_KEYS,
        &[
            ("dictionary", Node),
            ("id", Node),
            ("count", Num),
            ("begin", Num),
            ("kind", Sym),
            ("same-kind-count", Num),
            ("file", File),
        ],
        finish::Supplier::Provider,
    );
    run.declare(
        ASSIGNMENT_POSITIONS,
        &[("target", Node), ("value", Node), ("target-name", Sym), ("value-name", Sym),
          ("depth", Num), ("tuple-path", Num), ("later-target", Num), ("file", File)],
        provider,
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
    let delimiter_columns: Vec<_> = DELIMITER_COLUMNS
        .iter()
        .map(|&(label, kind)| (label, finish::ty_of(kind)))
        .collect();
    run.declare(NESTED_DELIMITERS, &delimiter_columns, provider);
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
    run.declare(BITWISE_OR_TREE, &[("root", Node), ("id", Node), ("left", Node), ("right", Node),
        ("names", Num), ("distinct-names", Num), ("nulls", Num), ("other", Num), ("file", File)], provider);
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
    file: &mut ColumnFile<'_, '_>,
    source: &[u8],
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
    } else if relation == DISPLAY_ELEMENT {
        file.visit_display_elements(|owner, id, position, count| {
            write(&[
                owner.0.into(),
                id.0.into(),
                position as u64,
                count as u64,
                (count - position - 1) as u64,
                file_id,
            ])
        });
    } else if relation == POSITIONAL_ARGUMENT {
        file.visit_positional_arguments(|owner, id, position| {
            write(&[owner.0.into(), id.0.into(), position as u64, file_id])
        });
    } else if relation == ATTRIBUTE_PREFIX {
        file.visit_attribute_prefixes(|root, end, depth| {
            write(&[root.0.into(), end.0.into(), depth as u64, file_id])
        });
    } else if relation == CONSTANT_PREDECESSOR {
        file.visit_repeated_constant_keys(|dict, id, previous| {
            write(&[dict.0.into(), id.0.into(), previous.0.into(), file_id])
        });
    } else if relation == ANNOTATION_STRINGS {
        file.visit_annotation_strings(|id, root| write(&[id.0.into(), root.0.into(), file_id]));
    } else if relation == SUBSCRIPT {
        file.visit_subscript_operands(|id, value, slice, name| {
            write(&[
                id.0.into(),
                value.0.into(),
                slice.0.into(),
                u64::from(run.sym(name.unwrap_or_default().as_bytes())),
                file_id,
            ])
        });
    } else if relation == SCHEDULE_END {
        if let Some(point) = file.binding_schedule_end()? {
            write(&[point, file_id]);
        }
    } else if relation == BITWISE_OR_TREE {
        file.visit_bitwise_or_trees(|root, id, left, right, names, distinct, nulls, other| {
            write(&[root.0.into(), id.0.into(), left.0.into(), right.0.into(), names as u64,
                distinct as u64, nulls as u64, other as u64, file_id]);
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
    } else if relation == WITH_HEADER {
        file.visit_with_headers(|id, colon| {
            let s = ryo::languages::node_table::span_of(file.table, colon);
            write(&[
                id.0.into(),
                colon.0.into(),
                (u64::from(s.start.0) << 32) | u64::from(s.end.0),
                file_id,
            ]);
        });
    } else if relation == COMPARISONS || relation == BOOLEANS {
        let operands = |id: Position, left: Position, right: Position, op: &str, count: usize| {
            write(&[
                id.0.into(),
                left.0.into(),
                right.0.into(),
                run.sym(op.as_bytes()).into(),
                count as u64,
                file_id,
            ]);
        };
        if relation == COMPARISONS {
            file.visit_comparison_operands(operands);
        } else {
            file.visit_boolean_operands(operands);
        }
    } else if relation == DICT_KEYS {
        file.visit_dictionary_keys(|dict, key, count, peers| {
            let begin = ryo::languages::node_table::span_of(file.table, key).start.0;
            write(&[
                dict.0.into(),
                key.0.into(),
                count as u64,
                begin.into(),
                run.kind_sym[usize::from(ryo::languages::node_table::kind_of(file.table, key).0)].into(),
                peers as u64,
                file_id,
            ]);
        });
    } else if relation == ASSIGNMENT_POSITIONS {
        file.visit_assignment_name_positions(|target, value, left, right, depth, tuples, later| {
            write(&[target.0.into(), value.0.into(), run.sym(left.as_bytes()).into(),
                run.sym(right.as_bytes()).into(), depth as u64, tuples.into(), later.into(), file_id]);
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
        let comma_kind = u64::from(run.sym(b"comma"));
        let name_kind = u64::from(run.sym(b"name-token"));
        let kind = |p| {
            u64::from(
                run.kind_sym[usize::from(ryo::languages::node_table::kind_of(file.table, p).0)],
            )
        };
        let mut pairs = Vec::new();
        file.visit_nested_delimiters(|outer, inner| pairs.push((outer, inner)));
        if !pairs.is_empty() {
            // The provider's token order supplies the same matching endpoints as token-link.
            #[derive(Default)]
            struct Interior {
                close: Option<Position>,
                comma: Option<Position>,
                yield_token: Option<Position>,
                commas: u64,
                yields: u64,
            }
            let mut interiors: rustc_hash::FxHashMap<u32, Interior> = pairs
                .iter()
                .map(|&(_, inner)| (inner.0, Interior::default()))
                .collect();
            let tokens = run.tokens.as_ref().ok_or("missing token order")?;
            // The lexical stream identifies each delimiter's matching end and its directly enclosed tokens.
            tokens.links(file.table, &mut |event| match event {
                finish::TokenEvent::Matching(open, close) => {
                    if let Some(slot) = interiors.get_mut(&open) {
                        slot.close = Some(Position(close));
                    }
                }
                finish::TokenEvent::Enclosing(id, open) => {
                    let Some(slot) = interiors.get_mut(&open) else {
                        return;
                    };
                    let position = Position(id);
                    let token_kind = kind(position);
                    if token_kind == comma_kind {
                        let _ = slot.comma.get_or_insert(position);
                        slot.commas += 1;
                    } else if token_kind == name_kind {
                        let span = ryo::languages::node_table::span_of(file.table, position);
                        if source.get(span.start.0 as usize..span.end.0 as usize) == Some(b"yield")
                        {
                            let _ = slot.yield_token.get_or_insert(position);
                            slot.yields += 1;
                        }
                    }
                }
                _ => {}
            });
            let value = |id: Option<Position>| id.map_or(finish::NULL, |p| u64::from(p.0));
            for (outer, inner) in pairs {
                let info = &interiors[&inner.0];
                let close = info.close;
                let nonempty = close
                    .is_some_and(|c| file.next_code_token(inner).is_some_and(|first| first != c));
                let after = close.and_then(|c| file.next_code_token(c));
                write(&[
                    outer.0.into(),
                    inner.0.into(),
                    kind(outer),
                    kind(inner),
                    nonempty.into(),
                    after.map_or(finish::NULL, kind),
                    value(close),
                    value(after),
                    value(info.comma),
                    value(info.yield_token),
                    info.commas,
                    info.yields,
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
