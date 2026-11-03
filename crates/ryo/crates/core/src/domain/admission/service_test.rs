//! The `admission` service's cases, on values alone.

use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder};
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, FileId, KindBits, Offset,
};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::FileBytes;

use super::errors::AdmissionError;
use super::service::{admit_column, admit_elements, admit_view, limits_of};
use super::types::{ByteLength, ElementTotal, Limits, StoredNumbers};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases offers to admission.
#[derive(Clone, Copy)]
enum Scenario {
    /// The cells of one column, as stored.
    Column(CellKind, &'static [u64]),
    /// A tree of one element that ends at the given byte.
    Element(u32),
    /// A parser-implied node without a source location.
    Implied,
    /// The rows of a relation, as the kinds of its columns and the cells of each.
    View(&'static [CellKind], &'static [&'static [u64]]),
    /// What the output for file 3 must lie within, for a tree of one element and a file of the given number of bytes.
    Limits(usize),
}

/// What a row of the cases offers to admission.
type Input = Scenario;
/// How many cells were admitted.
type Value = usize;
type Failure = AdmissionError;

/// The file visited is 3, its table has 10 elements, and it has 100 bytes.
const LIMITS: Limits = Limits {
    file: FileId(3),
    elements: ElementTotal(10),
    diagnostics: ElementTotal(2),
    length: ByteLength(100),
};

/// Spans stored as the start in the high half and the end in the low: within the file.
const SPANS_WITHIN: &[u64] = &[100, (5 << 32) | 5, (10 << 32) | 0x14];
/// A span that ends after the file.
const SPAN_PAST_THE_END: &[u64] = &[101];
/// A span that ends before it starts.
const SPAN_BACKWARDS: &[u64] = &[(10 << 32) | 5];
/// Nodes of file 3 at the first and the last place of its table.
const NODES_IN_THE_TABLE: &[u64] = &[0x0000_0003_0000_0000, 0x0000_0003_0000_0009];
/// A node of file 3 one place past its table.
const NODE_PAST_THE_TABLE: &[u64] = &[0x0000_0003_0000_000a];
/// The two reports of file 3, in the space of their own.
const REPORTS: &[u64] = &[0x0000_0003_8000_0000, 0x0000_0003_8000_0001];
/// A report of file 3 past the two there are.
const REPORT_PAST_THE_REPORTS: &[u64] = &[0x0000_0003_8000_0002];
/// A node of file 4.
const NODE_OF_ANOTHER_FILE: &[u64] = &[0x0000_0004_0000_0001];

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Column(kind, cells) => {
            admit_column(kind, &StoredNumbers(cells), &LIMITS).map(|()| cells.len())
        }
        Scenario::Implied => {
            let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
            let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b""));
            ryo_language::domain::elements::service::enter_implied_node(&mut builder, KindBits(0));
            leave_node(&mut builder, Offset(0));
            admit_elements(&elements, &LIMITS).map(|()| 1)
        }
        Scenario::Element(end) => {
            let mut elements = Elements::default();
            let mut symbols = Symbols::default();
            let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b""));
            enter_node(&mut builder, KindBits(0), Offset(0));
            leave_node(&mut builder, Offset(end));
            admit_elements(&elements, &LIMITS).map(|()| 1)
        }
        Scenario::View(kinds, columns) => {
            let columns: Vec<FactColumn> = columns
                .iter()
                .map(|cells| FactColumn(cells.to_vec()))
                .collect();
            admit_view(
                &ColumnsView {
                    kinds,
                    columns: &columns,
                },
                &LIMITS,
            )
            .map(|()| columns.len())
        }
        Scenario::Limits(length) => {
            let mut elements = Elements::default();
            let mut symbols = Symbols::default();
            let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b""));
            enter_node(&mut builder, KindBits(0), Offset(0));
            leave_node(&mut builder, Offset(1));
            let limits = limits_of(&elements, &FileBytes(vec![0; length]), FileId(3));
            let file = usize::try_from(limits.file.0).unwrap_or(usize::MAX);
            Ok(file * 1_000_000
                + usize::try_from(limits.elements.0).unwrap_or(usize::MAX) * 1000
                + usize::try_from(limits.length.0).unwrap_or(usize::MAX))
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("implied_node_has_no_fabricated_location"),
        word: Word::Succeeds,
        input: || Scenario::Implied,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("explicit_absent_span_is_admitted"),
        word: Word::Succeeds,
        input: || Scenario::Column(CellKind::Span, &[0xffff_ffff_0000_0000]),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("spans_within_the_file_are_admitted"),
        word: Word::Succeeds,
        input: || Scenario::Column(CellKind::Span, SPANS_WITHIN),
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("nodes_of_the_file_in_its_table_are_admitted"),
        word: Word::Succeeds,
        input: || Scenario::Column(CellKind::Node, NODES_IN_THE_TABLE),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("the_reports_of_the_file_are_admitted_in_their_own_space"),
        word: Word::Succeeds,
        input: || Scenario::Column(CellKind::Node, REPORTS),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("the_file_visited_is_admitted_as_a_file"),
        word: Word::Succeeds,
        input: || Scenario::Column(CellKind::File, &[3, 3]),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("numbers_the_file_does_not_prove_are_admitted"),
        word: Word::Succeeds,
        input: || Scenario::Column(CellKind::Number, &[u64::MAX]),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_element_within_the_file_is_admitted"),
        word: Word::Succeeds,
        input: || Scenario::Element(100),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_limits_of_a_file_are_its_number_its_elements_and_its_bytes"),
        word: Word::Succeeds,
        input: || Scenario::Limits(7),
        expect: Expect::Value(3_001_007),
    },
    Case {
        name: CaseName("every_column_of_a_relation_within_the_file_is_admitted"),
        word: Word::Succeeds,
        input: || {
            Scenario::View(
                &[CellKind::Span, CellKind::Node],
                &[SPANS_WITHIN, NODES_IN_THE_TABLE],
            )
        },
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_language_that_writes_a_node_past_its_file_is_refused_naming_it"),
        word: Word::Refuses,
        input: || Scenario::View(&[CellKind::Node], &[NODE_PAST_THE_TABLE]),
        expect: Expect::Error(AdmissionError::NodeOutsideTable),
    },
    Case {
        name: CaseName("a_column_after_a_good_one_is_checked_too"),
        word: Word::Refuses,
        input: || {
            Scenario::View(
                &[CellKind::Span, CellKind::Node],
                &[SPANS_WITHIN, NODE_OF_ANOTHER_FILE],
            )
        },
        expect: Expect::Error(AdmissionError::NodeOfAnotherFile),
    },
    Case {
        name: CaseName("a_span_that_ends_after_the_file_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Column(CellKind::Span, SPAN_PAST_THE_END),
        expect: Expect::Error(AdmissionError::SpanOutsideFile),
    },
    Case {
        name: CaseName("a_span_that_ends_before_it_starts_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Column(CellKind::Span, SPAN_BACKWARDS),
        expect: Expect::Error(AdmissionError::SpanOutsideFile),
    },
    Case {
        name: CaseName("a_node_past_the_table_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Column(CellKind::Node, NODE_PAST_THE_TABLE),
        expect: Expect::Error(AdmissionError::NodeOutsideTable),
    },
    Case {
        name: CaseName("a_report_past_the_reports_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Column(CellKind::Node, REPORT_PAST_THE_REPORTS),
        expect: Expect::Error(AdmissionError::NodeOutsideTable),
    },
    Case {
        name: CaseName("a_node_of_another_file_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Column(CellKind::Node, NODE_OF_ANOTHER_FILE),
        expect: Expect::Error(AdmissionError::NodeOfAnotherFile),
    },
    Case {
        name: CaseName("a_cell_naming_another_file_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Column(CellKind::File, &[3, 4]),
        expect: Expect::Error(AdmissionError::FileNotVisited),
    },
    Case {
        name: CaseName("an_element_that_ends_after_the_file_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Element(101),
        expect: Expect::Error(AdmissionError::SpanOutsideFile),
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
