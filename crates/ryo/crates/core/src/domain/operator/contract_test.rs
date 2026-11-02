//! Executable pilot conformance: real schema, compute, stream and engine absence policy.

use super::collections::operators::interface::OperatorsItem;
use super::service::pilot;
use super::types::{AbsencePolicy, RegisteredOperator};
use crate::domain::coverage::types::{Lacking, Partial};
use crate::domain::features::service::kind_of;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room, Supply};
use crate::domain::operation_schema::service::project_schema;
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::replay::types::Pipe;
use crate::domain::table_model::service::{push_row, reset};
use crate::domain::table_model::types::{RowCount, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use crate::domain::trust::service::{blocked, needs_of, scoped_needs};
use crate::domain::trust::types::Blocking;
use ryo_language::domain::relations::types::{CellKind, Integer, Offset, Relation, Span, Value};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, JoinColumn, LiteralText, RepeatedRows, Side, StepId,
};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{
    Emit as PlanEmit, Operation, OperationKind, Plan, PlanStep, StepScope,
};
use ryo_values::domain::stored_facts::types::PartialCause;

const SOURCE: &[u8] = b"a a";
const FROM: Input = Input::Relation(Relation::Function);
const RIGHT: Input = Input::Relation(Relation::Scope);

fn span() -> Span {
    Span {
        start: Offset(0),
        end: Offset(3),
    }
}

fn operation(kind: OperationKind) -> Operation {
    match kind {
        OperationKind::Select => Operation::Select {
            from: FROM,
            comparisons: vec![],
            kept: vec![],
        },
        OperationKind::Project => Operation::Project {
            from: FROM,
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
        OperationKind::SourceMatches => Operation::SourceMatches {
            from: FROM,
            span: ColumnIndex(0),
            base_line: None,
            pattern: LiteralText("a".into()),
            capture: None,
            capture_text: false,
            capture_line: false,
        },
        OperationKind::SpanBounds => Operation::SpanBounds {
            from: FROM,
            span: ColumnIndex(0),
        },
        OperationKind::Join => Operation::Join {
            left: FROM,
            right: RIGHT,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(0),
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        },
        OperationKind::Union => Operation::Union {
            left: FROM,
            right: RIGHT,
        },
        _ => panic!("unregistered pilot kind"),
    }
}

fn input(rows: usize) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Span],
        ..Table::default()
    };
    for _ in 0..rows {
        push_row(&mut table, &[Value::Span(span())]);
    }
    table
}

fn numbered(rows: usize) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for index in 0..rows {
        push_row(
            &mut table,
            &[Value::Number(Integer(
                i64::try_from(index).expect("bounded fixture"),
            ))],
        );
    }
    table
}

fn numbers(values: &[i64]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for value in values {
        push_row(&mut table, &[Value::Number(Integer(*value))]);
    }
    table
}

fn keyed_numbers(values: &[(i64, i64)]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    for (key, payload) in values {
        push_row(
            &mut table,
            &[
                Value::Number(Integer(*key)),
                Value::Number(Integer(*payload)),
            ],
        );
    }
    table
}

struct Held<'a> {
    tables: &'a [&'a Table],
    visits: [usize; 2],
}
impl<'f> Feed<'f> for Held<'f> {
    fn kinds(&self, place: InputPlace) -> Result<&[CellKind], OperatingError> {
        self.tables
            .get(place.0)
            .map(|table| table.kinds.as_slice())
            .ok_or(OperatingError::MissingInput {
                position: place.0 + 1,
            })
    }
    fn streams(&self, _: InputPlace) -> Supply {
        Supply::AtHand
    }
    fn found(&mut self, _: InputPlace, _: RowCount) {}
    fn stream(
        &mut self,
        place: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let table = self
            .tables
            .get(place.0)
            .ok_or(OperatingError::MissingInput {
                position: place.0 + 1,
            })?;
        for row in table.cells.chunks_exact(table.kinds.len()) {
            self.visits[place.0] += 1;
            emit.row(context, row)?;
        }
        Ok(())
    }
    fn hold<'t>(
        &mut self,
        place: InputPlace,
        _: &mut OperatorContext<'_>,
        _: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't,
    {
        let table = self
            .tables
            .get(place.0)
            .ok_or(OperatingError::MissingInput {
                position: place.0 + 1,
            })?;
        self.visits[place.0] += table.count.0;
        Ok(Rows::Computed(table))
    }
}

#[derive(Debug)]
struct Work {
    visits: [usize; 2],
    rows: usize,
    scratch_cells: usize,
    cells: Vec<Value>,
}

/// Return a failure reason, so a deliberately faulty kernel proves this check is active.
fn verify(
    reg: &RegisteredOperator,
    operation: &Operation,
    tables: &[&Table],
) -> Result<Work, &'static str> {
    let declared = (reg.input)(operation).map_err(|_| "input")?;
    if kind_of(operation) != reg.kind || declared != inputs_of(operation) {
        return Err("identity");
    }
    if named(&declared).len() != tables.len() || reg.absence != AbsencePolicy::PositiveRows {
        return Err("policy");
    }
    let plan = Plan {
        parameters: vec![],
        steps: vec![PlanStep {
            operation: operation.clone(),
            scope: StepScope::PerFile,
        }],
        emit: PlanEmit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    };
    let needs = needs_of(&plan);
    if needs.len() != 1 || needs.iter().any(|reads| !reads.is_empty()) {
        return Err("partial policy");
    }
    let scoped = scoped_needs(&plan);
    for input in named(&declared) {
        if let Input::Relation(relation) = input {
            let partial = Partial {
                relation: *relation,
                cause: PartialCause("fixture partial".into()),
                lacking: Lacking::Any,
                at: None,
            };
            if blocked(&needs[0], &[partial]) != Blocking::Free
                || !scoped.reads.iter().any(|read| read.relation == *relation)
            {
                return Err("partial distinction");
            }
        }
    }
    let input_schemas: Vec<_> = tables
        .iter()
        .map(|table| InputSchema {
            kinds: &table.kinds,
            tree: None,
        })
        .collect();
    let schema = (reg.schema)(operation, &input_schemas, &[]).map_err(|_| "schema")?;
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut computed = Table::default();
    let rows: Vec<_> = tables.iter().map(|table| Rows::Computed(table)).collect();
    reg.item
        .compute(operation, &rows, &mut context, &mut computed)
        .map_err(|_| "compute")?;
    if schema != computed.kinds {
        return Err("computed schema");
    }
    let mut held = Held {
        tables,
        visits: [0; 2],
    };
    let mut room = Room::default();
    let mut streamed = Vec::new();
    reg.item
        .stream(
            operation,
            &mut held,
            &mut context,
            &mut room,
            &mut Pipe(|_: &mut OperatorContext<'_>, row: &[Value]| {
                streamed.extend_from_slice(row);
                Ok(())
            }),
        )
        .map_err(|_| "stream")?;
    if computed.cells != streamed {
        return Err("ordered bag");
    }
    let width = computed.kinds.len();
    if width == 0 || computed.count.0 != streamed.len() / width {
        return Err("row count");
    }
    Ok(Work {
        visits: held.visits,
        rows: computed.count.0,
        scratch_cells: room.scratch.row.capacity() + room.scratch.other.capacity(),
        cells: computed.cells,
    })
}

#[test]
fn pilot_registrations_are_unique_and_refuse_another_variant() {
    let kinds = [
        OperationKind::Select,
        OperationKind::Project,
        OperationKind::Join,
        OperationKind::Union,
        OperationKind::SourceMatches,
        OperationKind::SpanBounds,
    ];
    let registrations: Vec<_> = kinds
        .iter()
        .map(|kind| pilot(&operation(*kind)).unwrap())
        .collect();
    assert_eq!(
        registrations.len(),
        registrations
            .iter()
            .map(|reg| reg.kind)
            .collect::<std::collections::HashSet<_>>()
            .len()
    );
    for (at, reg) in registrations.iter().enumerate() {
        let foreign = operation(kinds[(at + 1) % kinds.len()]);
        assert_eq!((reg.input)(&foreign), Err(OperatingError::WrongOperation));
        assert_eq!(
            (reg.schema)(&foreign, &[], &[]),
            Err(OperatingError::WrongOperation)
        );
        let mut symbols = Symbols::default();
        let mut scratch = OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(SOURCE),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        assert_eq!(
            reg.item
                .compute(&foreign, &[], &mut context, &mut Table::default()),
            Err(OperatingError::WrongOperation)
        );
    }
}

#[test]
fn pilot_registry_checks_real_ordered_bags_and_linear_visits() {
    for kind in [
        OperationKind::Select,
        OperationKind::Project,
        OperationKind::SourceMatches,
        OperationKind::SpanBounds,
    ] {
        let operation = self::operation(kind);
        let reg = pilot(&operation).expect("pilot registration");
        assert_eq!((reg.input)(&operation), Ok(inputs_of(&operation)));
        let empty = verify(reg, &operation, &[&input(0)]).unwrap();
        assert_eq!((empty.visits[0], empty.rows), (0, 0));
        let first = verify(reg, &operation, &[&input(32)]).unwrap();
        let second = verify(reg, &operation, &[&input(64)]).unwrap();
        assert!(first.visits[0] > 0, "the input is read");
        assert_eq!(second.visits[0], first.visits[0] * 2);
        assert_eq!(second.rows, first.rows * 2);
        assert_eq!(
            second.scratch_cells, first.scratch_cells,
            "scratch does not scale with input rows"
        );
    }
}

#[test]
fn binary_pilot_inputs_keep_order_and_scale_with_their_rows() {
    for kind in [OperationKind::Join, OperationKind::Union] {
        let operation = operation(kind);
        let reg = pilot(&operation).expect("binary pilot registration");
        assert_eq!((reg.input)(&operation), Ok(inputs_of(&operation)));
        let empty = numbered(0);
        let none = verify(reg, &operation, &[&empty, &empty]).unwrap();
        assert_eq!(none.rows, 0);
        assert_eq!(none.visits, [0, 0]);

        let small = numbered(32);
        let large = numbered(64);
        let first = verify(reg, &operation, &[&small, &small]).unwrap();
        let second = verify(reg, &operation, &[&large, &large]).unwrap();
        assert_eq!((first.rows, second.rows), (32, 64));
        for side in 0..2 {
            assert!(first.visits[side] > 0, "both inputs are read");
            assert_eq!(second.visits[side], first.visits[side] * 2);
        }
    }
}

#[test]
fn registered_select_join_and_union_preserve_exact_ordered_bags() {
    let selected = numbers(&[1, 1, 2]);
    let select = operation(OperationKind::Select);
    let select_reg = pilot(&select).unwrap();
    assert_eq!(
        verify(select_reg, &select, &[&selected]).unwrap().cells,
        selected.cells,
        "select keeps duplicate rows in input order"
    );

    let left = keyed_numbers(&[(1, 10), (1, 11), (2, 12)]);
    let right = keyed_numbers(&[(1, 20), (2, 21), (1, 22)]);
    let mut join = operation(OperationKind::Join);
    if let Operation::Join { output, .. } = &mut join {
        output[0].column = ColumnIndex(1);
        output[1].column = ColumnIndex(1);
    }
    let join_reg = pilot(&join).unwrap();
    assert_eq!(
        verify(join_reg, &join, &[&left, &right]).unwrap().cells,
        [10, 20, 10, 22, 11, 20, 11, 22, 12, 21]
            .map(|value| Value::Number(Integer(value)))
            .to_vec(),
        "join keeps left order and each matching right row's order"
    );

    let left = numbers(&[1, 1, 2]);
    let right = numbers(&[2, 3, 1]);
    let union = operation(OperationKind::Union);
    let union_reg = pilot(&union).unwrap();
    assert_eq!(
        verify(union_reg, &union, &[&left, &right]).unwrap().cells,
        [1, 2, 3]
            .map(|value| Value::Number(Integer(value)))
            .to_vec(),
        "union emits each distinct row at its first occurrence"
    );
}

#[test]
fn pilot_registry_preserves_bad_input_and_failing_emitter_prefix() {
    for kind in [
        OperationKind::Select,
        OperationKind::Project,
        OperationKind::SourceMatches,
        OperationKind::SpanBounds,
    ] {
        let mut operation = operation(kind);
        match &mut operation {
            Operation::Select { kept, .. } => kept.push(ColumnIndex(1)),
            Operation::Project { columns, .. } => columns[0] = ColumnIndex(1),
            _ => {}
        }
        let reg = pilot(&operation).unwrap();
        let mut malformed = Table {
            kinds: vec![CellKind::Number],
            ..Table::default()
        };
        push_row(&mut malformed, &[Value::Number(Integer(1))]);
        let schema_error = (reg.schema)(
            &operation,
            &[InputSchema {
                kinds: &malformed.kinds,
                tree: None,
            }],
            &[],
        )
        .unwrap_err();
        let mut symbols = Symbols::default();
        let mut scratch = OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(SOURCE),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        let compute_error = reg
            .item
            .compute(
                &operation,
                &[Rows::Computed(&malformed)],
                &mut context,
                &mut Table::default(),
            )
            .unwrap_err();
        let stream_error = reg
            .item
            .stream(
                &operation,
                &mut Held {
                    tables: &[&malformed],
                    visits: [0; 2],
                },
                &mut context,
                &mut Room::default(),
                &mut Pipe(|_: &mut OperatorContext<'_>, _: &[Value]| Ok(())),
            )
            .unwrap_err();
        assert_eq!(schema_error, compute_error);
        assert_eq!(compute_error, stream_error);

        let operation = self::operation(kind);
        let table = input(2);
        let full = verify(reg, &operation, &[&table]).unwrap();
        assert!(full.rows >= 2);
        let mut emitted = Vec::new();
        let stopped = reg.item.stream(
            &operation,
            &mut Held {
                tables: &[&table],
                visits: [0; 2],
            },
            &mut context,
            &mut Room::default(),
            &mut Pipe(|_: &mut OperatorContext<'_>, row: &[Value]| {
                if emitted.len() == 1 {
                    return Err(OperatingError::WrongOperation);
                }
                emitted.push(row.to_vec());
                Ok(())
            }),
        );
        assert_eq!(stopped, Err(OperatingError::WrongOperation));
        assert_eq!(
            emitted.len(),
            1,
            "first emitted row precedes the injected failure"
        );
        let mut expected = Table::default();
        reg.item
            .compute(
                &operation,
                &[Rows::Computed(&table)],
                &mut context,
                &mut expected,
            )
            .unwrap();
        assert_eq!(emitted[0], expected.cells[..expected.kinds.len()]);
    }
}

#[test]
fn binary_pilot_refuses_bad_shapes_and_preserves_emitter_prefix() {
    for kind in [OperationKind::Join, OperationKind::Union] {
        let mut invalid = operation(kind);
        let left = numbered(2);
        let right = if kind == OperationKind::Join {
            if let Operation::Join { left_on, .. } = &mut invalid {
                left_on[0] = ColumnIndex(1);
            }
            numbered(2)
        } else {
            input(2)
        };
        let reg = pilot(&invalid).unwrap();
        let schemas = [
            InputSchema {
                kinds: &left.kinds,
                tree: None,
            },
            InputSchema {
                kinds: &right.kinds,
                tree: None,
            },
        ];
        let schema_error = (reg.schema)(&invalid, &schemas, &[]).unwrap_err();
        let mut symbols = Symbols::default();
        let mut scratch = OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(SOURCE),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        let compute_error = reg
            .item
            .compute(
                &invalid,
                &[Rows::Computed(&left), Rows::Computed(&right)],
                &mut context,
                &mut Table::default(),
            )
            .unwrap_err();
        let stream_error = reg
            .item
            .stream(
                &invalid,
                &mut Held {
                    tables: &[&left, &right],
                    visits: [0; 2],
                },
                &mut context,
                &mut Room::default(),
                &mut Pipe(|_: &mut OperatorContext<'_>, _: &[Value]| Ok(())),
            )
            .unwrap_err();
        assert_eq!(schema_error, compute_error);
        assert_eq!(compute_error, stream_error);

        let valid = operation(kind);
        let right = numbered(2);
        let missing = OperatingError::MissingInput { position: 2 };
        assert_eq!(
            (reg.schema)(
                &valid,
                &[InputSchema {
                    kinds: &left.kinds,
                    tree: None
                }],
                &[],
            ),
            Err(missing.clone())
        );
        assert_eq!(
            reg.item.compute(
                &valid,
                &[Rows::Computed(&left)],
                &mut context,
                &mut Table::default(),
            ),
            Err(missing.clone())
        );
        assert_eq!(
            reg.item.stream(
                &valid,
                &mut Held {
                    tables: &[&left],
                    visits: [0; 2]
                },
                &mut context,
                &mut Room::default(),
                &mut Pipe(|_: &mut OperatorContext<'_>, _: &[Value]| Ok(())),
            ),
            Err(missing)
        );
        let full = verify(reg, &valid, &[&left, &right]).unwrap();
        assert_eq!(full.rows, 2);
        let mut emitted = Vec::new();
        let stopped = reg.item.stream(
            &valid,
            &mut Held {
                tables: &[&left, &right],
                visits: [0; 2],
            },
            &mut context,
            &mut Room::default(),
            &mut Pipe(|_: &mut OperatorContext<'_>, row: &[Value]| {
                if emitted.len() == 1 {
                    return Err(OperatingError::WrongOperation);
                }
                emitted.push(row.to_vec());
                Ok(())
            }),
        );
        assert_eq!(stopped, Err(OperatingError::WrongOperation));
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0], full.cells[..full.cells.len() / full.rows]);
    }
}

struct BrokenItem;
impl OperatorsItem for BrokenItem {
    fn compute(
        &self,
        _: &Operation,
        inputs: &[Rows<'_>],
        _: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        reset(
            out,
            &crate::domain::table_model::service::kinds_of(&inputs[0]),
        );
        Ok(())
    }
    fn stream(
        &self,
        _: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        _: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        ins.stream(InputPlace(0), context, emit)
    }
}
static BROKEN: BrokenItem = BrokenItem;

#[test]
fn conformance_rejects_a_deliberately_broken_kernel() {
    let real = pilot(&operation(OperationKind::Project)).unwrap();
    let broken = RegisteredOperator {
        kind: OperationKind::Project,
        item: &BROKEN,
        schema: project_schema,
        input: real.input,
        absence: AbsencePolicy::PositiveRows,
    };
    assert_eq!(
        verify(&broken, &operation(OperationKind::Project), &[&input(2)]).err(),
        Some("ordered bag")
    );
}
