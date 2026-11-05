//! Exact byte offsets, invalid ranges, absence and identical streaming results.
use crate::application::file_dag::feed::Feed;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, InputPlace, OperatorContext, Room, Supply};
use crate::domain::operator::collections::operators::span_between::item::SpanBetweenItem;
use crate::domain::replay::types::Pipe;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::types::{CellKind, Offset, Relation, Span, Value};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
use ryo_values::domain::plans::types::{Operation, QueryOperator};

type Input = (Span, Span, &'static [u8]);
type Failure = OperatingError;
const fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}

struct Held<'f>(
    &'f Table,
    std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
);
fn held(table: &Table) -> Held<'_> {
    Held(
        table,
        std::rc::Rc::new(crate::application::file_dag::relations::RelationOwner::new(
            true,
            crate::outbound::derived::factory::test_port(),
        )),
    )
}
impl<'f> Feed<'f> for Held<'f> {
    fn relation_owner(
        &self,
    ) -> std::rc::Rc<crate::application::file_dag::relations::RelationOwner> {
        std::rc::Rc::clone(&self.1)
    }
    fn origin(
        &self,
        which: InputPlace,
    ) -> Result<crate::application::file_dag::feed::InputOrigin, OperatingError> {
        crate::application::file_dag::testing::external_origin(which, 1)
    }
    fn publication(
        &self,
        which: InputPlace,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, OperatingError>
    {
        crate::application::file_dag::testing::no_publication(which, 1)
    }
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError> {
        crate::application::file_dag::testing::supplied(which, 1, Supply::AtHand)?;
        Ok(&self.0.kinds)
    }
    fn streams(&self, which: InputPlace) -> Result<Supply, OperatingError> {
        crate::application::file_dag::testing::supplied(which, 1, Supply::AtHand)
    }
    fn found(&mut self, _: InputPlace, _: RowCount) {}
    fn stream(
        &mut self,
        _: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        for row in self.0.cells.chunks_exact(2) {
            emit.row(context, row)?;
        }
        Ok(())
    }
    fn hold<'t>(
        &mut self,
        _: InputPlace,
        _: &mut OperatorContext<'_>,
        _: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't,
    {
        Ok(Rows::Computed(self.0))
    }
}

fn subject((left, right, source): Input) -> Result<Span, Failure> {
    let mut table = Table {
        kinds: vec![CellKind::Span, CellKind::Span],
        ..Table::default()
    };
    let row = [Value::Span(left), Value::Span(right)];
    push_row(&mut table, &row);
    push_row(&mut table, &row);
    let operation = Operation::SpanBetween {
        from: PlanInput::Relation(Relation::Function),
        start: ColumnIndex(0),
        end: ColumnIndex(1),
    };
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut scratch,
    };
    let mut output = Table::default();
    let computed = SpanBetweenItem.compute(
        &operation,
        &[Rows::Computed(&table)],
        &mut context,
        &mut output,
    );
    let mut streamed = Vec::new();
    let streaming = SpanBetweenItem.stream(
        &operation,
        &mut held(&table),
        &mut context,
        &mut Room::default(),
        &mut Pipe(|_: &mut OperatorContext<'_>, emitted: &[Value]| {
            streamed.extend_from_slice(emitted);
            Ok(())
        }),
    );
    assert_eq!(computed, streaming);
    computed?;
    assert_eq!(streamed, output.cells);
    assert_eq!(output.count.0, 2);
    assert_eq!(&output.cells[..2], &row);
    let Value::Span(result) = output.cells[2] else {
        panic!("span output required")
    };
    Ok(result)
}

const CASES: &[Case<Input, Span, Failure>] = &[
    Case {
        name: CaseName("suffix_contained_in_boolop"),
        word: Word::Succeeds,
        input: || (span(5, 6), span(0, 11), b"x or 1 or y"),
        expect: Expect::Value(span(5, 11)),
    },
    Case {
        name: CaseName("overlap"),
        word: Word::Succeeds,
        input: || (span(1, 5), span(3, 8), b"0123456789"),
        expect: Expect::Value(span(1, 8)),
    },
    Case {
        name: CaseName("nested_end_is_not_union"),
        word: Word::Succeeds,
        input: || (span(1, 9), span(3, 6), b"0123456789"),
        expect: Expect::Value(span(1, 6)),
    },
    Case {
        name: CaseName("gap"),
        word: Word::Succeeds,
        input: || (span(1, 2), span(7, 8), b"0123456789"),
        expect: Expect::Value(span(1, 8)),
    },
    Case {
        name: CaseName("equal"),
        word: Word::Succeeds,
        input: || (span(1, 3), span(1, 3), b"012"),
        expect: Expect::Value(span(1, 3)),
    },
    Case {
        name: CaseName("empty_file"),
        word: Word::Succeeds,
        input: || (span(0, 0), span(0, 0), b""),
        expect: Expect::Value(span(0, 0)),
    },
    Case {
        name: CaseName("eof"),
        word: Word::Succeeds,
        input: || (span(3, 3), span(3, 3), b"abc"),
        expect: Expect::Value(span(3, 3)),
    },
    Case {
        name: CaseName("unicode_multiline_byte_offsets"),
        word: Word::Succeeds,
        input: || (span(1, 2), span(3, 5), "é\nxy".as_bytes()),
        expect: Expect::Value(span(1, 5)),
    },
    Case {
        name: CaseName("absent_left"),
        word: Word::Succeeds,
        input: || (NO_SPAN, span(0, 1), b"a"),
        expect: Expect::Value(NO_SPAN),
    },
    Case {
        name: CaseName("absent_right"),
        word: Word::Succeeds,
        input: || (span(0, 1), NO_SPAN, b"a"),
        expect: Expect::Value(NO_SPAN),
    },
    Case {
        name: CaseName("both_absent"),
        word: Word::Succeeds,
        input: || (NO_SPAN, NO_SPAN, b""),
        expect: Expect::Value(NO_SPAN),
    },
    Case {
        name: CaseName("absent_still_checks_right"),
        word: Word::Refuses,
        input: || (NO_SPAN, span(2, 1), b"ab"),
        expect: Expect::Error(OperatingError::InvalidSpan),
    },
    Case {
        name: CaseName("absent_still_checks_left"),
        word: Word::Refuses,
        input: || (span(2, 1), NO_SPAN, b"ab"),
        expect: Expect::Error(OperatingError::InvalidSpan),
    },
    Case {
        name: CaseName("reversed_composition"),
        word: Word::Refuses,
        input: || (span(7, 8), span(1, 2), b"0123456789"),
        expect: Expect::Error(OperatingError::InvalidSpan),
    },
    Case {
        name: CaseName("out_of_file"),
        word: Word::Refuses,
        input: || (span(0, 4), span(0, 1), b"abc"),
        expect: Expect::Error(OperatingError::InvalidSpan),
    },
    Case {
        name: CaseName("max_offset"),
        word: Word::Refuses,
        input: || (span(0, u32::MAX), span(0, 1), b"abc"),
        expect: Expect::Error(OperatingError::InvalidSpan),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn contract() {
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::SpanBetween,
    );
}

use ryo_values::domain::rows::types::{RowCount, Table};
