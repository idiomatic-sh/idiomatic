//! The plans the load and the scan scenarios of the one lowering and the one executor name, each the smallest of its shape.

use super::parts::{at, joined, kept, of, plan, projected, step, without, words, words_written};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ComparisonOrigin, Direction, HostedNumber, Input, LiteralText, Operand, ParameterId,
    ParameterName, PlanComparison, QueryLiteral, Side, StepId, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::{ParameterType, PlanParameter};
use ryo_values::domain::query_plans::types::Plan;

/// The plans of the load scenarios of the one lowering, each the smallest of its shape; none for another name.
pub(super) fn load_scene(scene: &str) -> Option<Plan> {
    let words_of = Input::Step(StepId(0));
    Some(match scene {
        // A rule over the host's rows with no step that names their columns: it cannot be evaluated.
        "over the host's rows" => plan(vec![step(Operation::Hosted {
            relation: HostedNumber(0),
        })]),
        // The words `a` beside the words `b` of the file, joined on the file alone: no key within the file.
        "a product within a file" => plan(vec![
            words(),
            words_written("a", &[]),
            words_written("b", &[]),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(2)),
                (&[4], &[4]),
                vec![of(Side::Left, 1), of(Side::Right, 1), of(Side::Left, 4)],
            ),
        ]),
        // The words projected onto 65 columns: more than a word of bits.
        "a definition of 65 columns" => {
            let columns: Vec<u32> = (0..65).map(|column| column % 5).collect();
            plan(vec![
                words(),
                projected(0, &columns),
                kept(Input::Step(StepId(1))),
            ])
        }
        // The words projected onto 65,536 columns: more than a table's columns are counted in.
        "a step of 65,536 columns" => {
            let columns: Vec<u32> = (0..=u32::from(u16::MAX)).map(|column| column % 5).collect();
            plan(vec![words(), projected(0, &columns)])
        }
        // Two rules that share one sub-plan, the words, as the right side of an antijoin: the side of an antijoin that is
        // asked "is there a row" is a definition whatever its size (it is never flattened into what reads it), so the two
        // rules lower one definition between them. Each step is read once, and the rules differ in their own step alone.
        "the elements that are no word" => plan(vec![
            words(),
            without(Input::Relation(Relation::SyntaxTree), (&[0], &[1])),
        ]),
        "the edges to no word" => plan(vec![
            words(),
            without(Input::Relation(Relation::SyntaxChild), (&[1], &[1])),
        ]),
        // The edges to no element whose text is the rule's parameter: the right side of the antijoin names the parameter, so
        // two rules bound to two values lower two definitions, and no step without the parameter is one.
        "the edges to no word of a parameter" => {
            let mut bound = plan(vec![
                step(Operation::Select {
                    from: Input::Relation(Relation::SyntaxTree),
                    comparisons: vec![PlanComparison {
                        test: Test::Equal,
                        left: Operand::Column(at(2)),
                        right: Operand::Parameter(ParameterId(0)),
                        origin: ComparisonOrigin::Unrecorded,
                    }],
                    kept: Vec::new(),
                }),
                without(Input::Relation(Relation::SyntaxChild), (&[1], &[0])),
            ]);
            bound.parameters = vec![PlanParameter {
                name: ParameterName("word".into()),
                kind: ParameterType::Text,
            }];
            bound
        }
        // The words beside the words of their sentence, then the pairs whose first is the earlier: two reads and a test whose
        // order a sample's statistics may change.
        "two orders of one rule" => plan(vec![
            words(),
            joined(
                words_of,
                words_of,
                (&[0, 4], &[0, 4]),
                vec![
                    of(Side::Left, 2),
                    of(Side::Right, 2),
                    of(Side::Left, 1),
                    of(Side::Left, 4),
                ],
            ),
            step(Operation::Select {
                from: Input::Step(StepId(1)),
                comparisons: vec![PlanComparison {
                    test: Test::LessThan,
                    left: Operand::Column(at(0)),
                    right: Operand::Column(at(1)),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: vec![at(2), at(3)],
            }),
        ]),
        _ => return None,
    })
}

/// Every element of `tree` beside its parent by the edges of `edges`: a walk of the parent axis from each element.
fn parents(tree: Relation, edges: Relation) -> Plan {
    plan(vec![
        kept(Input::Relation(tree)),
        step(Operation::Axis {
            from: Input::Step(StepId(0)),
            edges: Input::Relation(edges),
            domain: Input::Relation(tree),
            direction: Direction::Parent,
            id: at(0),
            parent: at(0),
            child: at(1),
            domain_id: at(0),
            domain_filter: Vec::new(),
            domain_view: Vec::new(),
            output: Vec::new(),
        }),
    ])
}

/// The plans of the scan scenarios of the one executor, each the smallest of its shape; none for another name.
pub(super) fn scan_scene(scene: &str) -> Option<Plan> {
    let words_of = Input::Step(StepId(0));
    let pairs = |filter: Option<PlanStep>| {
        let mut steps = vec![
            words(),
            joined(
                words_of,
                words_of,
                (&[3, 4], &[3, 4]),
                vec![
                    of(Side::Left, 2),
                    of(Side::Right, 2),
                    of(Side::Left, 1),
                    of(Side::Left, 4),
                ],
            ),
        ];
        steps.extend(filter);
        plan(steps)
    };
    Some(match scene {
        // The words beside the syntax errors of their file: a file with no error has an empty side.
        "join with an empty side" => plan(vec![
            words(),
            joined(
                words_of,
                Input::Relation(Relation::SyntaxError),
                (&[4], &[5]),
                vec![of(Side::Left, 1), of(Side::Left, 4)],
            ),
        ]),
        // The words `one` beside the words `two` on their element: both sides have rows, no pair has one element.
        "join empty by its keys" => plan(vec![
            words(),
            words_written("one", &[]),
            words_written("two", &[]),
            joined(
                Input::Step(StepId(1)),
                Input::Step(StepId(2)),
                (&[1], &[1]),
                vec![of(Side::Left, 1), of(Side::Left, 4)],
            ),
        ]),
        // The count of the words of each text, read once by the words and once by the words `a`.
        "one table, two readers" => plan(vec![
            words(),
            step(Operation::GroupCount {
                from: words_of,
                keys: vec![at(3), at(4)],
                distinct: None,
            }),
            joined(
                words_of,
                Input::Step(StepId(1)),
                (&[3, 4], &[0, 1]),
                vec![of(Side::Left, 1), of(Side::Left, 4)],
            ),
            words_written("a", &[]),
            joined(
                Input::Step(StepId(3)),
                Input::Step(StepId(1)),
                (&[3, 4], &[0, 1]),
                vec![of(Side::Left, 1), of(Side::Left, 4)],
            ),
            step(Operation::Union {
                left: Input::Step(StepId(2)),
                right: Input::Step(StepId(4)),
            }),
        ]),
        // Each word beside the words of its text, asked once by each of two words `a`: one key, two callers.
        "one definition, two callers" => pairs(None),
        // The words `two!` beside the syntax errors on their element: a read of the filled relation by one key.
        "keyed read" => plan(vec![
            words(),
            joined(
                words_of,
                Input::Relation(Relation::SyntaxError),
                (&[1], &[0]),
                vec![of(Side::Left, 1), of(Side::Left, 4)],
            ),
        ]),
        // The links of the file, every one: a read of the whole relation.
        "every link" => plan(vec![kept(Input::Relation(Relation::TokenLink))]),
        // The words beside the links that start at them: a read of the links by one key, their source, for each word.
        "the links of the words" => plan(vec![
            words(),
            joined(
                words_of,
                Input::Relation(Relation::TokenLink),
                (&[1], &[0]),
                vec![of(Side::Left, 1), of(Side::Left, 4)],
            ),
        ]),
        // Each element beside its parent, walked over the file's own tree and over the tree that holds the embedded segments.
        "the parents over the syntax tree" => parents(Relation::SyntaxTree, Relation::SyntaxChild),
        "the parents over the semantic tree" => {
            parents(Relation::SemanticSyntax, Relation::SemanticChild)
        }
        // The child edges whose field is named `label`: the field column holds a field's name, never its code.
        "the edges of the field label" => plan(vec![step(Operation::Select {
            from: Input::Relation(Relation::SyntaxChild),
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(at(3)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("label".into()))),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: Vec::new(),
        })]),
        // Each word beside each word of its text that comes after it: the pairs of one group, a quarter of its square.
        "pairs within one group" => pairs(Some(step(Operation::Select {
            from: Input::Step(StepId(1)),
            comparisons: vec![PlanComparison {
                test: Test::LessThan,
                left: Operand::Column(at(0)),
                right: Operand::Column(at(1)),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: vec![at(2), at(3)],
        }))),
        _ => return None,
    })
}
