//! What a file of the activation target holds besides its tree: each fact relation's rows and the relations read in part.
use ryo_execution::inbound::use_cases::execution::interface::Partial;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value,
};

/// What the file holds besides its tree: the rows of each fact relation, and the relations read only in part.
#[derive(Debug, Default)]
pub(crate) struct Given {
    /// Each relation's rows; a relation not here is refused.
    pub(crate) facts: Vec<(Relation, ryo_values::domain::rows::types::Table)>,
    /// The relations read in part, any row possibly missing.
    pub(crate) partials: Vec<Partial>,
}

/// `count` rows of `Function`, each a node of the file.
fn functions(count: u32) -> ryo_values::domain::rows::types::Table {
    let kinds = ryo_relations::domain::relations::fixtures::kinds(Relation::Function);
    let mut cells = Vec::new();
    for at in 0..count {
        cells.extend(kinds.iter().map(|kind| match kind {
            CellKind::Node => Value::Node(ryo_relations::domain::relations::fixtures::node_id(
                FileId(0),
                Position(at),
            )),
            CellKind::Symbol => Value::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
            CellKind::File => Value::File(FileId(0)),
            CellKind::Span => Value::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
            CellKind::Number => Value::Number(Integer(0)),
            CellKind::Flag => Value::Flag(Flag::Lowered),
        }));
    }
    ryo_values::domain::rows::types::Table {
        kinds: kinds.to_vec(),
        cells,
        count: ryo_values::domain::rows::types::RowCount(usize::try_from(count).unwrap_or(0)),
    }
}

/// No fact relation: a read of one is refused.
pub(crate) fn no_facts() -> Given {
    Given::default()
}

/// Two rows of `Function`.
pub(crate) fn two_functions() -> Given {
    Given {
        facts: vec![(Relation::Function, functions(2))],
        partials: vec![],
    }
}

/// No row of `Function`.
pub(crate) fn no_function() -> Given {
    Given {
        facts: vec![(Relation::Function, functions(0))],
        partials: vec![],
    }
}

/// No row of `Function`, read in part: a row may be missing.
pub(crate) fn no_function_read_in_part() -> Given {
    Given {
        facts: vec![(Relation::Function, functions(0))],
        partials: vec![Partial {
            relation: Relation::Function,
            cause: ryo_values::domain::stored_facts::types::PartialCause("a reader stopped".into()),
            lacking: ryo_execution::inbound::use_cases::metadata::interface::Lacking::Any,
            at: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(FileId(
                0,
            )),
        }],
    }
}

/// Two rows of `Function` and one of `Comment`.
pub(crate) fn functions_and_comments() -> Given {
    let comments = {
        let kinds = ryo_relations::domain::relations::fixtures::kinds(Relation::Comment);
        ryo_values::domain::rows::types::Table {
            kinds: kinds.to_vec(),
            cells: kinds
                .iter()
                .map(|kind| match kind {
                    CellKind::Node => Value::Node(
                        ryo_relations::domain::relations::fixtures::node_id(FileId(0), Position(9)),
                    ),
                    CellKind::Symbol => {
                        Value::Symbol(ryo_relations::domain::terms::types::Symbol(0))
                    }
                    CellKind::File => Value::File(FileId(0)),
                    CellKind::Span => Value::Span(Span {
                        start: Offset(0),
                        end: Offset(0),
                    }),
                    CellKind::Number => Value::Number(Integer(0)),
                    CellKind::Flag => Value::Flag(Flag::Lowered),
                })
                .collect(),
            count: ryo_values::domain::rows::types::RowCount(1),
        }
    };
    Given {
        facts: vec![
            (Relation::Function, functions(2)),
            (Relation::Comment, comments),
        ],
        partials: vec![],
    }
}
