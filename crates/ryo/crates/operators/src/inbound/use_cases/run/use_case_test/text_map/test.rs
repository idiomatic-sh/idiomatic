//! The `text_map` kernel's cases, through the plan and run use cases: each text kind's checks in the interpreter's order, its
//! symbols interned or minted, and its pattern compiled once per worker per node.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::fixtures::{Step, defines_rows, over_defines, over_nodes};
use crate::domain::physical_plan::types::PlanNodeId;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::inbound::use_cases::run::fixtures::counting::CountingMatching;
use crate::inbound::use_cases::run::fixtures::texts::{
    SOURCE, absent, answer, literal, plain, replace, search, shown, span,
};
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources, emitted_through};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, JoinColumn, ParameterId, Side,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::{MaxLine, TabWidth, TextOutput};
use std::borrow::Cow;
use std::sync::Arc;

/// What a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    AbsentSpanInvalidPattern,
    TextWithoutCapture,
    MissingCapture,
    InternedAndMinted,
    CompiledOnce,
    ReplaceOfNoText,
    SpanBetween,
    Ranked,
    UnboundRank,
    UnboundRankOverNoRows,
    Fits,
    EarlierRowWins,
}
type Input = Scenario;
/// The answer as text: the emitted rows, or the fault; and what the case counted.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("source_matches_checks_the_span_before_the_pattern"),
        word: Word::Succeeds,
        input: || Scenario::AbsentSpanInvalidPattern,
        expect: Expect::Value(Cow::Borrowed("0 rows ")),
    },
    Case {
        name: CaseName("source_matches_refuses_outputs_without_a_capture_on_the_first_row"),
        word: Word::Succeeds,
        input: || Scenario::TextWithoutCapture,
        expect: Expect::Value(Cow::Borrowed("WrongOperation")),
    },
    Case {
        name: CaseName("a_missing_capture_names_it"),
        word: Word::Succeeds,
        input: || Scenario::MissingCapture,
        expect: Expect::Value(Cow::Borrowed(
            "MissingCapture { capture: LiteralText(\"name\") }",
        )),
    },
    Case {
        name: CaseName("captured_text_is_interned_and_replaced_text_minted"),
        word: Word::Succeeds,
        input: || Scenario::InternedAndMinted,
        expect: Expect::Value(Cow::Borrowed(
            "1 rows [Span(Span { start: Offset(6), end: Offset(7) }), Span(Span { start: Offset(6), end: Offset(7) }), Symbol(Symbol(6))]; 1 rows [Symbol(Symbol(7))]; 1 rows [Symbol(Symbol(0))]; 1 rows [Symbol(Symbol(8))]; [Interned(\"a\"), Minted(\"c_nst\"), Minted(\"const\"), Minted(\"nst\")]",
        )),
    },
    Case {
        name: CaseName("a_pattern_is_compiled_once_per_worker_per_node"),
        word: Word::Succeeds,
        input: || Scenario::CompiledOnce,
        expect: Expect::Value(Cow::Borrowed(
            "3 rows [Symbol(Symbol(6))] [Symbol(Symbol(5))] [Symbol(Symbol(6))]; 3 rows [Symbol(Symbol(6))] [Symbol(Symbol(5))] [Symbol(Symbol(6))]; 1 compiles",
        )),
    },
    Case {
        name: CaseName("replace_refuses_a_cell_that_is_not_text_before_compiling"),
        word: Word::Succeeds,
        input: || Scenario::ReplaceOfNoText,
        expect: Expect::Value(Cow::Borrowed("ShapeMismatch")),
    },
    Case {
        name: CaseName("span_between_answers_ecmascript_spans"),
        word: Word::Succeeds,
        input: || Scenario::SpanBetween,
        expect: Expect::Value(Cow::Borrowed(
            "Ok([[Span(Span { start: Offset(0), end: Offset(5) }), Span(Span { start: Offset(0), end: Offset(5) }), Span(Span { start: Offset(0), end: Offset(5) }), Span(Span { start: Offset(4294967295), end: Offset(0) }), Span(Span { start: Offset(4294967295), end: Offset(0) }), Span(Span { start: Offset(4294967295), end: Offset(0) })]])",
        )),
    },
    Case {
        name: CaseName("rank_appends_the_first_layer_and_drops_the_unranked"),
        word: Word::Succeeds,
        input: || Scenario::Ranked,
        expect: Expect::Value(Cow::Borrowed(
            "2 rows [Number(Integer(1))] [Number(Integer(0))]",
        )),
    },
    Case {
        name: CaseName("an_unbound_rank_refuses_at_its_first_row"),
        word: Word::Succeeds,
        input: || Scenario::UnboundRank,
        expect: Expect::Value(Cow::Borrowed("Unbound")),
    },
    Case {
        name: CaseName("an_unbound_rank_over_no_rows_answers_none"),
        word: Word::Succeeds,
        input: || Scenario::UnboundRankOverNoRows,
        expect: Expect::Value(Cow::Borrowed("0 rows ")),
    },
    Case {
        name: CaseName("fits_rendered_counts_the_line_prefix_and_tab_stops"),
        word: Word::Succeeds,
        input: || Scenario::Fits,
        expect: Expect::Value(Cow::Borrowed("0 rows ; 1 rows []")),
    },
    Case {
        name: CaseName("a_text_map_fault_on_an_earlier_row_wins"),
        word: Word::Succeeds,
        input: || Scenario::EarlierRowWins,
        expect: Expect::Value(Cow::Borrowed(
            "Pattern(InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" })",
        )),
    },
];

#[expect(clippy::too_many_lines, reason = "one arm per case of the kernel")]
fn subject(scenario: Input) -> Result<Value, Failure> {
    let ranks = || {
        over_defines(Operation::Rank {
            from: StepInput::Relation(Relation::Defines),
            column: ColumnIndex(3),
            sets: ParameterId(0),
        })
    };
    Ok(Cow::Owned(match scenario {
        Scenario::AbsentSpanInvalidPattern => plain(
            &[search("[", None, TextOutput::Omitted)],
            &[("ab", absent())],
        ),
        Scenario::TextWithoutCapture => plain(
            &[search("a", None, TextOutput::Appended)],
            &[("ab", span(99, 100))],
        ),
        Scenario::MissingCapture => plain(
            &[search("(?P<x>a)", Some("name"), TextOutput::Omitted)],
            &[("ab", span(6, 7))],
        ),
        Scenario::InternedAndMinted => {
            let steps = [
                search("(?P<word>a)", Some("word"), TextOutput::Appended),
                replace(3, "o"),
                over_defines(Operation::Words {
                    from: StepInput::Relation(Relation::Defines),
                    text: ColumnIndex(3),
                }),
                over_defines(Operation::Capture {
                    from: StepInput::Relation(Relation::Defines),
                    path: ColumnIndex(3),
                    pattern: ryo_values::domain::plan_parts::types::CapturePattern(
                        "co{rest}".into(),
                    ),
                }),
            ];
            let mut resources = TextResources::new(SOURCE);
            let rows = [("const", span(0, 7))];
            let named: Vec<_> = rows
                .iter()
                .map(|(name, at)| (resources.symbol(name), *at))
                .collect();
            let table = defines_rows(&named);
            let inputs = TableInputs::new(&[(Relation::Defines, &table)]);
            let emits = [PlanNodeId(0), PlanNodeId(1), PlanNodeId(2), PlanNodeId(3)];
            let answered = emitted_through(
                &steps,
                &emits,
                (&[], Arc::new(CountingMatching::default())),
                &inputs,
                &mut resources,
                &mut FileBuffers::default(),
            );
            format!("{}; {:?}", shown(answered), resources.calls())
        }
        Scenario::CompiledOnce => {
            let matching = Arc::new(CountingMatching::default());
            let mut buffers = FileBuffers::default();
            let steps = [replace(3, "o")];
            let rows = [
                ("const", span(0, 5)),
                ("ab", span(0, 1)),
                ("const", span(0, 5)),
            ];
            let mut resources = TextResources::new(SOURCE);
            let first = answer(
                &steps,
                &rows,
                &[],
                Arc::clone(&matching),
                &mut resources,
                &mut buffers,
            );
            buffers.release(PlanNodeId(0));
            let second = answer(
                &steps,
                &rows,
                &[],
                Arc::clone(&matching),
                &mut resources,
                &mut buffers,
            );
            format!("{first}; {second}; {} compiles", matching.compiles())
        }
        Scenario::ReplaceOfNoText => plain(&[replace(0, "[")], &[("ab", span(0, 1))]),
        Scenario::SpanBetween => {
            let joined = over_nodes(
                Operation::Join {
                    left: StepInput::Relation(Relation::Defines),
                    right: StepInput::Relation(Relation::Defines),
                    left_on: vec![ColumnIndex(3)],
                    right_on: vec![ColumnIndex(3)],
                    output: vec![
                        JoinColumn {
                            side: Side::Left,
                            column: ColumnIndex(6),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(6),
                        },
                    ],
                    filter: Vec::new(),
                    view: Vec::new(),
                    reach: None,
                },
                &[],
            );
            let joined = Step {
                inputs: vec![
                    crate::domain::physical_plan::types::PlanInput::Relation(Relation::Defines),
                    crate::domain::physical_plan::types::PlanInput::Relation(Relation::Defines),
                ],
                ..joined
            };
            let between = over_nodes(
                Operation::SpanBetween {
                    from: StepInput::Step(ryo_values::domain::plan_parts::types::StepId(0)),
                    start: ColumnIndex(0),
                    end: ColumnIndex(1),
                },
                &[0],
            );
            let mut resources = TextResources::new(SOURCE);
            let rows = [("const", span(0, 5)), ("ab", absent())];
            let named: Vec<_> = rows
                .iter()
                .map(|(name, at)| (resources.symbol(name), *at))
                .collect();
            let table = defines_rows(&named);
            let inputs = TableInputs::new(&[(Relation::Defines, &table)]);
            let steps = [joined, between];
            let answered = emitted_through(
                &steps,
                &[PlanNodeId(1)],
                (&[], Arc::new(CountingMatching::default())),
                &inputs,
                &mut resources,
                &mut FileBuffers::default(),
            );
            format!(
                "{:?}",
                answered.map(|tables| tables
                    .into_iter()
                    .map(|table| table.cells)
                    .collect::<Vec<_>>())
            )
        }
        Scenario::Ranked => {
            let layers = [ParameterBinding::TextSets(vec![
                vec![literal("1")],
                vec![literal("ab")],
            ])];
            let mut resources = TextResources::new(SOURCE);
            let rows = [("ab", span(0, 1)), ("const", span(0, 1)), ("1", span(0, 1))];
            answer(
                &[ranks()],
                &rows,
                &layers,
                Arc::default(),
                &mut resources,
                &mut FileBuffers::default(),
            )
        }
        Scenario::UnboundRank => plain(&[ranks()], &[("ab", span(0, 1))]),
        Scenario::UnboundRankOverNoRows => plain(&[ranks()], &[]),
        Scenario::Fits => {
            let fits = |max_line: i64| {
                over_defines(Operation::FitsRendered {
                    from: StepInput::Relation(Relation::Defines),
                    texts: vec![ColumnIndex(3)],
                    literals: vec![literal(""), literal("")],
                    span: ColumnIndex(6),
                    max_line: MaxLine(max_line),
                    tab_width: TabWidth(8),
                })
            };
            // `ab` starts after a tab: the prefix reaches column 8, and the candidate `ab` two more.
            let rows = [("ab", span(14, 16))];
            format!(
                "{}; {}",
                plain(&[fits(9)], &rows),
                plain(&[fits(10)], &rows)
            )
        }
        Scenario::EarlierRowWins => {
            // The search fails on the second row; the replacement after it fails on the first row's occurrence, which wins.
            let replaced = over_nodes(
                Operation::Replace {
                    from: StepInput::Step(ryo_values::domain::plan_parts::types::StepId(0)),
                    text: ColumnIndex(3),
                    pattern: literal("["),
                    replacement: literal("_"),
                },
                &[0],
            );
            let steps = [search("o", None, TextOutput::Omitted), replaced];
            plain(&steps, &[("const", span(0, 5)), ("ab", span(99, 100))])
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
