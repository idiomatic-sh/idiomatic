//! The pool's records, the need records and the small codes: each written and read back through the one place that does both.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::records::service::{
    body_at, call_entry_place, need_at, next_place, operand_at, push_call_body, push_inline_body,
    push_list, push_need, push_operand, push_table_body,
};
use crate::domain::code::records::types::{BoundFormal, CollectedRead, NeedRecord, PooledOperand};
use crate::domain::code::service::{list, word};
use crate::domain::code::small_codes::service::{
    comparison_code, comparison_of, comparison_regex, context_code, context_of, direction_code,
    direction_of, meaning_code, meaning_flag, meaning_of, meaning_regex, mode_code, mode_of,
    node_column_code, node_column_of, side_code, side_of,
};
use crate::domain::code::types::{
    CodeEntry, ConstantNumber, IndexSite, MemoSite, NeedSite, Op, PoolAt, RelationNumber, Short,
    WordFlag, WordSlot,
};
use crate::domain::steps::calls::fixtures::searches;
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{
    AxisDirection, CallMeaning, MadeColumn, MadeTableId, RegexId, Register, TestOperation,
};
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::NodeColumn;
use ryo_values::domain::plan_orders::types::OrderBound;
use std::borrow::Cow;

/// What a row writes and reads back.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// An operand of each tag.
    StepOperands,
    /// A list, and a place the pool does not reach.
    Lists,
    /// A body record of each kind, one after the other.
    Bodies,
    /// A need record of each kind.
    Needs,
    /// Every value of every small code.
    SmallCodes,
    /// A number no value of a small code has.
    UnknownCodes,
    /// A word from its slots.
    WordSlots,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_operand_of_each_tag_reads_back"),
        word: Word::Succeeds,
        input: || Scenario::StepOperands,
        expect: Expect::Value(Cow::Borrowed(
            "[Some(Register(Register(70000))), Some(Constant(ConstantNumber(3))), Some(Constants(PoolAt(9))), Some(File)]; past the pool: None",
        )),
    },
    Case {
        name: CaseName("a_list_reads_back_and_a_place_the_pool_does_not_reach_is_empty"),
        word: Word::Succeeds,
        input: || Scenario::Lists,
        expect: Expect::Value(Cow::Borrowed(
            "[WordSlot(1), WordSlot(2), WordSlot(3)] at PoolAt(1); past the pool: []",
        )),
    },
    Case {
        name: CaseName("each_body_record_reads_back_with_the_place_after_it"),
        word: Word::Succeeds,
        input: || Scenario::Bodies,
        expect: Expect::Value(Cow::Borrowed(
            "starts PoolAt(1) PoolAt(3) PoolAt(12), the call's entry at PoolAt(4), ends PoolAt(22) of 22: Inline { entry: CodeEntry(4) }; Call { entry: CodeEntry(0), memo: MemoSite(2), unit: WordSlot(4294967295), need: NeedSite(6), bound: WordSlot(1), formals: PoolAt(9) }; Table { index: IndexSite(5), bound: WordSlot(2), values: PoolAt(15), collected: WordSlot(1), reads: PoolAt(20) }",
        )),
    },
    Case {
        name: CaseName("each_need_record_reads_back"),
        word: Word::Succeeds,
        input: || Scenario::Needs,
        expect: Expect::Value(Cow::Borrowed(
            "Some(Unknown); Some(Kinds(PoolAt(6))); Some(Row { relation: RelationNumber(2), column: WordSlot(1), constant: WordSlot(4) }); Some(Site(NeedSite(3))); Some(Table(MadeTableId(8))); Some(All { parts: WordSlot(0), offsets: WordSlot(13) }); Some(Any { parts: WordSlot(0), offsets: WordSlot(15) })",
        )),
    },
    Case {
        name: CaseName("every_value_of_every_small_code_reads_back"),
        word: Word::Succeeds,
        input: || Scenario::SmallCodes,
        expect: Expect::Value(Cow::Borrowed(
            "10 of 10 node columns, 16 of 16 comparisons, 19 of 19 meanings, 3 of 3 contexts, 5 of 5 directions, 4 of 4 sides, 3 of 3 modes, 12 name a regular expression",
        )),
    },
    Case {
        name: CaseName("a_number_no_small_code_has_reads_as_none"),
        word: Word::Succeeds,
        input: || Scenario::UnknownCodes,
        expect: Expect::Value(Cow::Borrowed("None None None None None None None")),
    },
    Case {
        name: CaseName("a_word_holds_each_operand_in_its_own_slot"),
        word: Word::Succeeds,
        input: || Scenario::WordSlots,
        expect: Expect::Value(Cow::Borrowed(
            "CodeWord { op: Test, flag: WordFlag(1), short: Short(2), a: WordSlot(3), b: WordSlot(4), x: WordSlot(5) }",
        )),
    },
];

/// The four operands, written one after the other and read back.
fn operands() -> String {
    let written = [
        PooledOperand::Register(Register(70_000)),
        PooledOperand::Constant(ConstantNumber(3)),
        PooledOperand::Constants(PoolAt(9)),
        PooledOperand::File,
    ];
    let mut pool = Vec::new();
    for operand in written {
        push_operand(&mut pool, operand);
    }
    let read: Vec<_> = (0..4)
        .map(|place| operand_at(&pool, PoolAt(place * 2)))
        .collect();
    format!(
        "{read:?}; past the pool: {:?}",
        operand_at(&pool, next_place(&pool))
    )
}

/// The three body records, written one after the other and read back each from where the one before it ended.
fn bodies() -> String {
    let mut pool = vec![WordSlot(99)];
    let first = push_inline_body(&mut pool, CodeEntry(4));
    let bound = [BoundFormal {
        formal: Register(7),
        value: PooledOperand::Register(Register(1)),
    }];
    let second = push_call_body(&mut pool, CodeEntry(0), MemoSite(2), NeedSite(6), &bound);
    let reads = [CollectedRead {
        column: MadeColumn(3),
        out: Register(8),
    }];
    let third = push_table_body(
        &mut pool,
        IndexSite(5),
        &[PooledOperand::File, PooledOperand::File],
        &reads,
    );
    let (mut at, mut read) = (first, Vec::new());
    while let Some(body) = body_at(&pool, at) {
        read.push(format!("{:?}", body.record));
        at = body.after;
    }
    format!(
        "starts {first:?} {second:?} {third:?}, the call's entry at {:?}, ends {at:?} of {}: {}",
        call_entry_place(second),
        pool.len(),
        read.join("; ")
    )
}

/// The seven need records, written and read back.
fn needs() -> String {
    let unset = NO_SLOT;
    let written = [
        NeedRecord::Unknown,
        NeedRecord::Kinds(PoolAt(6)),
        NeedRecord::Row {
            relation: RelationNumber(2),
            column: WordSlot(1),
            constant: WordSlot(4),
        },
        NeedRecord::Site(NeedSite(3)),
        NeedRecord::Table(MadeTableId(8)),
        NeedRecord::All {
            parts: WordSlot(0),
            offsets: unset,
        },
        NeedRecord::Any {
            parts: WordSlot(0),
            offsets: unset,
        },
    ];
    let mut records = Vec::new();
    let places: Vec<WordSlot> = written
        .iter()
        .map(|record| push_need(&mut records, *record))
        .collect();
    let read: Vec<String> = places
        .iter()
        .map(|place| format!("{:?}", need_at(&records, *place)))
        .collect();
    read.join("; ")
}

/// How many values of each small code read back as themselves, of how many.
fn small_codes() -> String {
    let regex = RegexId(6);
    let columns = [
        NodeColumn::Id,
        NodeColumn::Kind,
        NodeColumn::Parent,
        NodeColumn::Field,
        NodeColumn::Ordinal,
        NodeColumn::Start,
        NodeColumn::End,
        NodeColumn::Span,
        NodeColumn::File,
        NodeColumn::Text,
    ];
    let comparisons = [
        TestOperation::Equal,
        TestOperation::NotEqual,
        TestOperation::Less,
        TestOperation::LessOrEqual,
        TestOperation::Greater,
        TestOperation::GreaterOrEqual,
        TestOperation::In,
        TestOperation::NotIn,
        TestOperation::Matches(regex),
        TestOperation::Glob,
        TestOperation::NotGlob,
        TestOperation::StartsWith,
        TestOperation::EndsWith,
        TestOperation::NumericAtLeast,
        TestOperation::NaturalLess,
        TestOperation::Successor,
    ];
    let mut meanings = vec![
        CallMeaning::SpanBetween,
        CallMeaning::SpanBounds,
        CallMeaning::Capture,
        CallMeaning::Replace(regex),
        CallMeaning::Words,
        CallMeaning::Rank,
        CallMeaning::Concat,
        CallMeaning::Lower,
        CallMeaning::FitsRendered,
    ];
    meanings.extend(
        searches()
            .into_iter()
            .map(|search| CallMeaning::SourceMatches(regex, search)),
    );
    let contexts = [
        RenderContext::AssignmentTarget,
        RenderContext::AttributeReceiver,
        RenderContext::PositionalArgument,
    ];
    let directions = [
        AxisDirection::Parent,
        AxisDirection::Ancestor,
        AxisDirection::Descendant,
        AxisDirection::NextSibling,
        AxisDirection::PreviousSibling,
    ];
    let sides = [
        OrderBound::Before,
        OrderBound::AtOrBefore,
        OrderBound::After,
        OrderBound::AtOrAfter,
    ];
    let modes = [UseMode::Rows, UseMode::Present, UseMode::Absent];
    let same = |read: usize, all: usize, named: &str| format!("{read} of {all} {named}");
    let regexes = comparisons
        .iter()
        .filter_map(|test| comparison_regex(*test))
        .count()
        + meanings
            .iter()
            .filter_map(|meaning| meaning_regex(*meaning))
            .count();
    [
        same(
            columns
                .iter()
                .filter(|column| node_column_of(node_column_code(**column)) == Some(**column))
                .count(),
            columns.len(),
            "node columns",
        ),
        same(
            comparisons
                .iter()
                .filter(|test| comparison_of(comparison_code(**test), regex) == Some(**test))
                .count(),
            comparisons.len(),
            "comparisons",
        ),
        same(
            meanings
                .iter()
                .filter(|meaning| {
                    let (code, flag) = (meaning_code(**meaning), meaning_flag(**meaning));
                    meaning_of(code, regex, flag) == Some(**meaning)
                })
                .count(),
            meanings.len(),
            "meanings",
        ),
        same(
            contexts
                .iter()
                .filter(|context| context_of(context_code(**context)) == Some(**context))
                .count(),
            contexts.len(),
            "contexts",
        ),
        same(
            directions
                .iter()
                .filter(|direction| direction_of(direction_code(**direction)) == Some(**direction))
                .count(),
            directions.len(),
            "directions",
        ),
        same(
            sides
                .iter()
                .filter(|side| side_of(side_code(**side)) == Some(**side))
                .count(),
            sides.len(),
            "sides",
        ),
        same(
            modes
                .iter()
                .filter(|mode| mode_of(mode_code(**mode)) == Some(**mode))
                .count(),
            modes.len(),
            "modes",
        ),
        format!("{regexes} name a regular expression"),
    ]
    .join(", ")
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match scenario {
        Scenario::StepOperands => operands(),
        Scenario::Lists => {
            let mut pool = vec![WordSlot(0)];
            let at = push_list(&mut pool, &[WordSlot(1), WordSlot(2), WordSlot(3)]);
            format!(
                "{:?} at {at:?}; past the pool: {:?}",
                list(&pool, WordSlot(at.0)),
                list(&pool, WordSlot(99))
            )
        }
        Scenario::Bodies => bodies(),
        Scenario::Needs => needs(),
        Scenario::SmallCodes => small_codes(),
        Scenario::UnknownCodes => format!(
            "{:?} {:?} {:?} {:?} {:?} {:?} {:?}",
            node_column_of(Short(10)),
            comparison_of(Short(16), RegexId(0)),
            meaning_of(Short(10), RegexId(0), WordFlag(0)),
            context_of(Short(3)),
            direction_of(WordFlag(5)),
            side_of(WordFlag(4)),
            mode_of(WordFlag(3)),
        ),
        Scenario::WordSlots => {
            format!(
                "{:?}",
                word(
                    Op::Test,
                    WordFlag(1),
                    Short(2),
                    WordSlot(3),
                    WordSlot(4),
                    WordSlot(5)
                )
            )
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
