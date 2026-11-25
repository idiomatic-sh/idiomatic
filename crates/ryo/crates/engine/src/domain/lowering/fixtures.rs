//! The test support of the `lowering` concept: the plans the engine's seam scenarios name, built by hand (the engine reads no query
//! text). Each is the smallest plan of its shape; the lowering's track makes each faithful to the shape it names. The plan of
//! each operation's growth case is in `growth`.

mod flags;
pub mod growth;
mod observed;
mod parts;
pub mod pending;
mod scenes;
mod sets;
mod shapes;
mod views;

use parts::{
    as_of, at, axis, joined, kept, nearest, nearest_step, of, plan, step, words, words_written,
};
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_orders::types::{GroupEnd, OrderBound};
use ryo_values::domain::plan_parts::types::{
    Applied, CapturePattern, ColumnIndex, ComparisonOrigin, Direction, Function, Input,
    LiteralText, Operand, PlanComparison, QueryLiteral, ReachLookup, RepeatedRows, Side, StepId,
    StepScope, TableKey, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_values::domain::query_plans::service::{node_digests, reach_unfolded};
use ryo_values::domain::query_plans::types::{NodeBytes, NodeDigest, NodeDigests, Plan};
use ryo_values::domain::written_rows::types::{RowsSource, WrittenRow, WrittenRows};

/// A digest function for the engine's tests: the 128-bit FNV-1a fold of the bytes given. The engine hashes nothing, so its
/// tests stand in for the host's hash with this.
fn folded(bytes: NodeBytes<'_>) -> NodeDigest {
    let mut hash: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for byte in bytes.0 {
        hash = (hash ^ u128::from(*byte)).wrapping_mul(0x0100_0000_0000_0000_0000_013b);
    }
    NodeDigest(hash.to_le_bytes())
}

/// The digest of each step of `plan` under `bindings`, as a host gives them to `load`: by the tests' stand-in for its hash.
#[must_use]
pub fn digests_of(plan: &Plan, bindings: RuleBindings<'_>) -> NodeDigests {
    node_digests(plan, bindings, &folded)
}

/// The plans of the operations' scan scenarios, each over the words of the scanned file (step 0); none for another name.
fn operation_scene(scene: &str) -> Option<Plan> {
    let words_of = Input::Step(StepId(0));
    Some(match scene {
        // Each word with the word of the same text latest before it: two key groups in a file of two texts.
        "as-of latest before" => plan(vec![
            words(),
            as_of((0, &[3, 4]), (0, &[3, 4], 2), OrderBound::Before),
        ]),
        // The right rows are each word beside each word of its text, ordered by the first word's ordinal: words of one text
        // tie at each order, and every tied row answers.
        "as-of tied events" => plan(vec![
            words(),
            joined(
                words_of,
                words_of,
                (&[3, 4], &[3, 4]),
                vec![of(Side::Left, 2), of(Side::Right, 1), of(Side::Left, 4)],
            ),
            as_of((0, &[4]), (1, &[2], 0), OrderBound::Before),
        ]),
        // Each word `a` with the word `b` earliest after it.
        "as-of earliest after" => plan(vec![
            words(),
            words_written("a", &[]),
            words_written("b", &[]),
            as_of((1, &[4]), (2, &[4], 2), OrderBound::After),
        ]),
        // Each word beside each word of its text, by the other word's ordinal: the greatest ordinal of a text is held by one
        // row for each of its words.
        "group-max ties" => plan(vec![
            words(),
            joined(
                words_of,
                words_of,
                (&[3, 4], &[3, 4]),
                vec![
                    of(Side::Left, 1),
                    of(Side::Right, 2),
                    of(Side::Left, 3),
                    of(Side::Left, 4),
                ],
            ),
            step(Operation::GroupBound {
                from: Input::Step(StepId(1)),
                keys: vec![at(2), at(3)],
                order: at(1),
                end: GroupEnd::Greatest,
            }),
        ]),
        // The words placed by ordinal within the file, and the ones whose place is 0: the first.
        "group-ordinal from zero" => plan(vec![
            words(),
            step(Operation::GroupOrdinal {
                from: words_of,
                keys: vec![at(4)],
                order: at(2),
            }),
            step(Operation::Select {
                from: Input::Step(StepId(1)),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(at(5)),
                    right: Operand::Literal(QueryLiteral::Number(Integer(0))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            }),
        ]),
        // The sentence points to each of its words, and each word `a` to itself: the sentence reaches an `a` and an `a`
        // answers itself; the walk to any other word runs out of edges.
        "follow fixed points" => plan(vec![
            words(),
            step(Operation::Project {
                from: words_of,
                columns: vec![at(0), at(1), at(4)],
                distinct: RepeatedRows::Kept,
            }),
            words_written("a", &[1, 1, 4]),
            step(Operation::Union {
                left: Input::Step(StepId(1)),
                right: Input::Step(StepId(2)),
            }),
            step(Operation::Follow {
                edges: Input::Step(StepId(3)),
                keys: vec![at(2)],
                source: at(0),
                destination: at(1),
            }),
        ]),
        // The sentence points to each of its words: every element reached from an element over those edges.
        "closure of the sentence's edges" => plan(vec![
            words(),
            step(Operation::Project {
                from: words_of,
                columns: vec![at(0), at(1)],
                distinct: RepeatedRows::Dropped,
            }),
            step(Operation::Closure {
                edges: Input::Step(StepId(1)),
                keys: Vec::new(),
                source: at(0),
                destination: at(1),
            }),
        ]),
        // Each scope beside every scope it is within, as today's compiler writes it (a join that holds a reach over the
        // scopes) and as the engine receives it: the reach unfolded into a closure step appended after the join.
        "the scopes above, a reach join unfolded" => {
            let folded = plan(vec![
                kept(Input::Relation(Relation::Scope)),
                step(Operation::Join {
                    left: Input::Step(StepId(0)),
                    right: Input::Relation(Relation::Scope),
                    left_on: vec![at(1), at(2)],
                    right_on: vec![at(1), at(0)],
                    output: vec![of(Side::Left, 1), of(Side::Right, 2)],
                    filter: Vec::new(),
                    view: Vec::new(),
                    reach: Some(ReachLookup {
                        source: at(1),
                        destination: at(0),
                        keys: vec![at(2)],
                        columns: vec![at(0), at(1), at(2)],
                    }),
                }),
            ]);
            reach_unfolded(&folded).into_owned()
        }
        // Every scope a scope is within: the closure of the relation's own rows, from the enclosed scope to the enclosing.
        "closure of the scopes" => plan(vec![step(Operation::Closure {
            edges: Input::Relation(Relation::Scope),
            keys: vec![at(2)],
            source: at(1),
            destination: at(0),
        })]),
        // Each element beside the nearest element above it, climbed over the file's own tree.
        "the nearest element above, over the tree" => {
            nearest(Input::Relation(Relation::SyntaxChild))
        }
        // The same, climbed over edges a step derives: the child edges, kept as they are.
        "the nearest element above, over a step's edges" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            kept(Input::Relation(Relation::SyntaxChild)),
            nearest_step(Input::Step(StepId(1))),
        ]),
        // Each word beside its element's span, the tree read through a view of its id and span: the key and the answer
        // name the view's columns.
        "the words' spans through a view" => plan(vec![
            words(),
            step(Operation::Join {
                left: words_of,
                right: Input::Relation(Relation::SyntaxTree),
                left_on: vec![at(0)],
                right_on: vec![at(0)],
                output: vec![of(Side::Left, 0), of(Side::Right, 1)],
                filter: Vec::new(),
                view: vec![at(0), at(4)],
                reach: None,
            }),
        ]),
        // Each element beside its parent, the domain read through a view of its span and id: the entity is the view's
        // second column.
        "the parents through a view" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            axis(
                Input::Relation(Relation::SyntaxChild),
                Direction::Parent,
                Input::Relation(Relation::SyntaxTree),
                vec![at(4), at(0)],
            ),
        ]),
        // Each occurrence of `a` in an element's source, with the group `g` of it and the group's text.
        "the occurrences in the elements' source" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            step(Operation::SourceMatches {
                from: words_of,
                span: at(4),
                base_line: None,
                pattern: LiteralText("(?P<g>a)".into()),
                capture: Some(LiteralText("g".into())),
                capture_text: TextOutput::Appended,
                capture_line: LineOutput::Omitted,
            }),
        ]),
        // Each word's text taken apart by a pattern of two slots.
        "the words' texts captured" => plan(vec![
            words(),
            step(Operation::Capture {
                from: words_of,
                path: at(3),
                pattern: CapturePattern("{head}.{*rest}".into()),
            }),
        ]),
        // Each word paired with the next word of its text: the pairs of an ordered match, the answer one column of each side.
        "the words matched in order" => plan(vec![
            words(),
            step(Operation::OrderedMatch {
                left: words_of,
                right: words_of,
                left_on: vec![at(3)],
                right_on: vec![at(3)],
                left_order: at(2),
                right_order: at(2),
                output: vec![of(Side::Left, 0), of(Side::Right, 0)],
            }),
        ]),
        // Each element with the span from its own start to its own end.
        "the elements' spans between" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            step(Operation::SpanBetween {
                from: words_of,
                start: at(4),
                end: at(4),
            }),
        ]),
        // The words whose text after `x.` is `x.a`: a comparison of a function's text.
        "the words whose prefixed text is x.a" => plan(vec![
            words(),
            step(Operation::Select {
                from: words_of,
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Function(Applied {
                        function: Function::Concat,
                        arguments: vec![
                            Operand::Literal(QueryLiteral::Text(LiteralText("x.".into()))),
                            Operand::Column(at(3)),
                        ],
                    }),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("x.a".into()))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            }),
        ]),
        // Each word with its text, every `a` of it replaced by `b`.
        "the words' texts replaced" => plan(vec![
            words(),
            step(Operation::Replace {
                from: words_of,
                text: at(3),
                pattern: LiteralText("a".into()),
                replacement: LiteralText("b".into()),
            }),
        ]),
        // Each element with where its span starts and ends.
        "the elements' span bounds" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            step(Operation::SpanBounds {
                from: words_of,
                span: at(4),
            }),
        ]),
        // Two rows written in the query: made once for the run, after the files, so the scan of a file finds none of them.
        "written rows" => plan(vec![PlanStep {
            operation: Operation::Rows {
                source: RowsSource::Written(WrittenRows(vec![
                    WrittenRow(vec![LiteralText("a".into())]),
                    WrittenRow(vec![LiteralText("b".into())]),
                ])),
                keys: vec![TableKey("text".into())],
            },
            scope: StepScope::CrossFile,
        }]),
        _ => return None,
    })
}


/// The plan a seam scenario names; an unknown name is the one tree rule.
#[must_use]
pub fn named(scene: &str) -> Plan {
    if let Some(plan) = operation_scene(scene) {
        return plan;
    }
    let listed = scenes::load_scene(scene)
        .or_else(|| scenes::scan_scene(scene))
        .or_else(|| shapes::shape_scene(scene))
        .or_else(|| sets::set_scene(scene))
        .or_else(|| views::view_scene(scene))
        .or_else(|| flags::flag_scene(scene))
        .or_else(|| observed::observed_scene(scene));
    if let Some(plan) = listed {
        return plan;
    }
    match scene {
        // An antijoin whose right side reads a variable only the left binds.
        "unbound correlated sub-plan" => plan(vec![
            kept(Input::Relation(Relation::SyntaxTree)),
            kept(Input::Relation(Relation::SyntaxChild)),
            PlanStep {
                operation: Operation::Antijoin {
                    left: Input::Step(StepId(0)),
                    right: Input::Step(StepId(1)),
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(1)],
                },
                scope: StepScope::PerFile,
            },
        ]),
        // A chain longer than the lowering's 4,096-instruction budget.
        "program over budget" => plan(
            std::iter::once(kept(Input::Relation(Relation::SyntaxTree)))
                .chain((0..4_096_u32).map(|step| kept(Input::Step(StepId(step)))))
                .collect(),
        ),
        // A rule reading two relations, one of which no enabled program reads in the file.
        "two relations, one read" => plan(vec![
            kept(Input::Relation(Relation::SyntaxError)),
            kept(Input::Relation(Relation::Function)),
        ]),
        // A rule reading the relation the fixture language fills only in part.
        "partial relation" => plan(vec![kept(Input::Relation(Relation::SyntaxError))]),
        _ => plan(vec![kept(Input::Relation(Relation::SyntaxTree))]),
    }
}
