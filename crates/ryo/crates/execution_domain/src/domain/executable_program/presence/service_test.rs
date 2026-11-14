// ignore: idiomatic.arch/file-length - one case table for the one presence fold: every case shares its plan fixtures and helpers, and the fold has no parts of its own to split along.
//! The presence fold: every operation's declaration, the failure term of each prepared contract, the column origins a
//! test is grouped by, and a region table total over the compiled program.
use super::service::{ALWAYS, for_regions, of_graph};
use super::types::{FormulaId, GraphPresence, PresenceFormula, PresenceTable};
use crate::domain::file_program::types::RequestGraph;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::presence::{
    types::InputMask, types::PresenceMask, types::RowFailure,
};
use crate::domain::operator::contract::service::{prepare_for, semantics};
use crate::domain::operator::contract::types::EmptyRepresentation;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as Source, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, ReachLookup, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_operators::types::{LineOutput, OperationKind, TextOutput};
use std::borrow::Cow;

fn equal(column: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}
fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}
fn select(from: Source, comparisons: Vec<PlanComparison>) -> PlanStep {
    step(Operation::Select {
        from,
        comparisons,
        kept: vec![],
    })
}
fn at(step: u32) -> Source {
    Source::Step(StepId(step))
}
fn plan(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId(u32::try_from(steps.len() - 1).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
    }
}
fn graph(
    plans: &[Plan],
) -> (
    RequestGraph,
    crate::domain::file_program::types::RequestOperators,
) {
    let operators = crate::domain::file_program::service::operators::service::prepare(
        plans,
        &vec![vec![]; plans.len()],
        &[],
        &|relation| kinds(relation),
        &mut |_| Ok(()),
    )
    .expect("the plans prepare");
    let graph = crate::domain::file_program::service::compile_prepared(plans, &operators);
    (graph, operators)
}
fn fold(plans: &[Plan]) -> GraphPresence {
    of_graph(
        &graph(plans).0,
        &|_| super::types::RelationScope::CurrentFile,
        ryo_operators::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
        &ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers::default(),
    )
}
/// The skip formula of the last step of the first plan.
fn last(presence: &GraphPresence, plans: &[Plan]) -> FormulaId {
    let (graph, _) = graph(plans);
    let node = graph.program_nodes[0]
        .last()
        .and_then(|node| node.as_ref().ok())
        .expect("the last step has a node");
    presence.nodes[node.0]
}
/// The tests a formula ANDs, as (relation, comparisons), when it is a conjunction of tests.
fn conjuncts(table: &PresenceTable, formula: FormulaId) -> Vec<(Relation, Vec<PlanComparison>)> {
    let test = |id: &super::types::TestId| {
        let test = &table.tests[id.0];
        (test.relation, test.comparisons.to_vec())
    };
    match &table.formulas[formula.0] {
        PresenceFormula::Test(id) => vec![test(id)],
        PresenceFormula::All(parts) => parts
            .iter()
            .flat_map(|part| conjuncts(table, *part))
            .collect(),
        PresenceFormula::Always | PresenceFormula::Any(_) => vec![],
    }
}
fn replace(column: u32, kinds: &[CellKind], scope: StepScope) -> RowFailure {
    prepare_for(
        OperationKind::Replace,
        &Operation::Replace {
            from: Source::Relation(Relation::Comment),
            text: ColumnIndex(column),
            pattern: LiteralText("a".into()),
            replacement: LiteralText("b".into()),
        },
        &[InputShape { kinds, tree: None }],
        &[],
        scope,
    )
    .map(|contract| contract.failure)
    .unwrap_or(RowFailure::OnRows(InputMask(0)))
}

/// A source search over `span` of `from`, with an optional named capture.
fn search(from: Source, span: u32, pattern: &str, capture: Option<&str>) -> PlanStep {
    step(Operation::SourceMatches {
        from,
        span: ColumnIndex(span),
        base_line: None,
        pattern: LiteralText(pattern.into()),
        capture: capture.map(|name| LiteralText(name.into())),
        capture_text: TextOutput::Omitted,
        capture_line: LineOutput::Omitted,
    })
}
fn project(from: Source, columns: &[u32]) -> PlanStep {
    step(Operation::Project {
        from,
        columns: columns.iter().copied().map(ColumnIndex).collect(),
        distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
    })
}
/// The fold with `shared` read as a relation of the project, not the file, and the request's slots declared from the
/// plans, as the program compiler declares them.
fn fold_sharing(plans: &[Plan], shared: Option<Relation>) -> GraphPresence {
    let prepared =
        ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers::new(plans);
    of_graph(
        &graph(plans).0,
        &|relation| {
            if Some(relation) == shared {
                super::types::RelationScope::Shared
            } else {
                super::types::RelationScope::CurrentFile
            }
        },
        ryo_operators::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
        &prepared,
    )
}
/// The provider span columns the fold relies on, as (relation, columns).
fn relied(presence: &GraphPresence) -> Vec<(Relation, Vec<u32>)> {
    presence
        .table
        .spans
        .iter()
        .map(|check| {
            (
                check.relation,
                check.columns.iter().map(|column| column.0).collect(),
            )
        })
        .collect()
}

/// The top-level shape of a skip formula, with the parts it ANDs or ORs.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    Always,
    Test,
    All(usize),
    Any(usize),
}

/// One comparison of a presence test, as a row writes it: the test, the column on its left and the
/// text literal on its right.
#[derive(Clone, Debug, PartialEq)]
struct Compared {
    test: Test,
    column: Option<u32>,
    text: Option<Cow<'static, str>>,
}

/// A presence test as a row writes it: the relation it reads and its comparisons.
type Tested = (Relation, Cow<'static, [Compared]>);

/// The provider span columns a fold relies on, as (relation, columns).
type Relied = Cow<'static, [(Relation, Cow<'static, [u32]>)]>;

/// What the fold answers for the last step of a plan: whether its skip formula is the `Always` the
/// table starts with, the formula's shape, the tests it ANDs that compare anything, and the span
/// columns the fold relies on.
#[derive(Debug, PartialEq)]
struct Skip {
    always: bool,
    shape: Shape,
    tests: Cow<'static, [Tested]>,
    relied: Relied,
}

/// The operation kinds, and those whose declared presence is not the one listed for them: both
/// inputs for a join, an intersection and an ordered match, either input for a union, none for rows
/// and hosted, and the first input for every other kind.
#[derive(Debug, PartialEq)]
struct Declarations {
    kinds: usize,
    differing: Cow<'static, [OperationKind]>,
}

/// An operation's prepared empty-result gates: whether its presence requires all of a mask, how many
/// gates are on any representation, and how many of those gate on an input outside the mask.
#[derive(Debug, PartialEq)]
struct Gates {
    requires_all: bool,
    any: usize,
    outside_the_mask: usize,
}

/// Where a region's formula comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Taken {
    /// The formula of the plan's one node.
    Node,
    /// `Always`.
    Always,
    /// Some other formula.
    Other,
}

/// The region table of three regions over the plan's one node: where each region's formula comes
/// from, whether the node's own formula is `Always`, and the relations the first region reads.
#[derive(Debug, PartialEq)]
struct Regions {
    regions: Cow<'static, [Taken]>,
    node_always: bool,
    reads: Cow<'static, [Relation]>,
}

/// The fault catalogue's audit: whether each list is in declaration order, the faults a row-fallible
/// raiser does not declare, the faults a source search raises on rows and those a located proof
/// covers, the inputs an oversized literal pattern makes fallible, and two verdicts.
#[derive(Debug, PartialEq)]
struct Verdicts {
    errors_in_order: bool,
    faults_in_order: bool,
    undeclared: Cow<'static, [Cow<'static, str>]>,
    searched: Cow<'static, [Cow<'static, str>]>,
    proven: Cow<'static, [Cow<'static, str>]>,
    oversized: Option<InputMask>,
    not_run: Fault,
    missing_capture: Fault,
}

/// A source search a row prepares, over column `span` of the comment relation.
#[derive(Clone, Copy, Debug)]
struct Searching {
    span: u32,
    base_line: Option<u32>,
    capture: Option<&'static str>,
    text: TextOutput,
    scope: StepScope,
}

/// The first step of a plan whose second selects `never` from its column 1.
#[derive(Clone, Copy, Debug)]
enum Opening {
    /// A search of the comment span with a pattern too large to compile.
    Oversized,
    /// A search naming a capture its pattern does not have.
    MissingCapture,
    /// A search naming the capture its pattern has.
    NamedCapture,
    /// A search appending captured text with no capture.
    RowError,
}

/// The plans a fold row folds.
#[derive(Clone, Copy, Debug)]
enum Plans {
    /// A search over the comment span, then a select of `never`.
    ProviderSpan,
    /// An opening step, then a select of `never`.
    Opened(Opening),
    /// A search of the source location span from its start line, then a select.
    Numbered,
    /// The noqa grammar: two head searches unioned, a codes search and a token search.
    Noqa,
    /// A union of a comment span and a source location span, searched, the source location shared.
    MixedShared,
    /// The same union with nothing shared.
    MixedBoth,
    /// A self join of syntax tree rows on column 3, selected for `x` on the left and `z` on the right.
    SelfJoin,
    /// A join of comments to syntax tree rows with this view, reach and filter, selected for `x`.
    JoinRight {
        view: bool,
        reach: bool,
        filter: bool,
    },
    /// A union of two selects of syntax tree rows, selected for `z`.
    Union,
    /// Two cross-file selects.
    CrossFile,
    /// A select on a column the syntax tree does not have, then a select.
    UnknownColumn,
    /// The span bounds of syntax tree rows, selected for `never`.
    FallibleUpstream,
}

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// Every operation kind's declared presence.
    Declarations,
    /// The failure of a replace over column `column` of a node, a symbol and a file column.
    Replace(u32, StepScope),
    /// The failure of a row-fallible operator over two function inputs.
    Fallible(OperationKind),
    /// The failure of a source search.
    Search(Searching),
    /// The fold of the plans.
    Fold(Plans),
    /// The empty-result gates of an operation over three syntax child inputs.
    Gates(OperationKind),
    /// The region table of a one-select plan.
    Regions,
    /// The fault catalogue's audit.
    Verdicts,
}

/// What a row answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Declarations(Declarations),
    Failure(RowFailure),
    Skip(Skip),
    Gates(Gates),
    Regions(Regions),
    Verdicts(Verdicts),
}

type Input = Asked;
type Value = Answer;
type Failure = OperatingError;

fn subject(asked: Input) -> Result<Value, Failure> {
    Ok(match asked {
        Asked::Declarations => Answer::Declarations(declarations()),
        Asked::Replace(column, scope) => Answer::Failure(replace(
            column,
            &[CellKind::Node, CellKind::Symbol, CellKind::File],
            scope,
        )),
        Asked::Fallible(kind) => Answer::Failure(fallible(kind)?),
        Asked::Search(searching) => Answer::Failure(searched(searching)?),
        Asked::Fold(plans) => Answer::Skip(folded(plans)),
        Asked::Gates(kind) => Answer::Gates(gates(kind)?),
        Asked::Regions => Answer::Regions(regions()),
        Asked::Verdicts => Answer::Verdicts(verdicts()),
    })
}

fn declarations() -> Declarations {
    use OperationKind::*;
    let differing = OperationKind::ALL
        .iter()
        .copied()
        .filter(|kind| {
            let listed = match kind {
                Join | Intersection | OrderedMatch => PresenceMask::AllOf(InputMask(3)),
                Union => PresenceMask::AnyOf(InputMask(3)),
                Rows | Hosted => PresenceMask::Always,
                _ => PresenceMask::AllOf(InputMask(1)),
            };
            semantics(*kind).presence != listed
        })
        .collect();
    Declarations {
        kinds: OperationKind::ALL.len(),
        differing,
    }
}

fn fallible(kind: OperationKind) -> Result<RowFailure, OperatingError> {
    let kinds = kinds(Relation::Function);
    let shape = [
        InputShape { kinds, tree: None },
        InputShape { kinds, tree: None },
    ];
    let function = Source::Relation(Relation::Function);
    let operation = if kind == OperationKind::OrderedMatch {
        Operation::OrderedMatch {
            left: function,
            right: function,
            left_on: vec![],
            right_on: vec![],
            left_order: ColumnIndex(0),
            right_order: ColumnIndex(0),
            output: vec![],
        }
    } else if kind == OperationKind::SpanBounds {
        Operation::SpanBounds {
            from: function,
            span: ColumnIndex(0),
        }
    } else {
        Operation::Select {
            from: function,
            comparisons: vec![],
            kept: vec![],
        }
    };
    prepare_for(kind, &operation, &shape, &[], StepScope::PerFile).map(|contract| contract.failure)
}

fn searched(searching: Searching) -> Result<RowFailure, OperatingError> {
    let kinds = kinds(Relation::Comment);
    let shape = [InputShape { kinds, tree: None }];
    let operation = Operation::SourceMatches {
        from: Source::Relation(Relation::Comment),
        span: ColumnIndex(searching.span),
        base_line: searching.base_line.map(ColumnIndex),
        pattern: LiteralText("a".into()),
        capture: searching.capture.map(|name| LiteralText(name.into())),
        capture_text: searching.text,
        capture_line: LineOutput::Omitted,
    };
    prepare_for(
        OperationKind::SourceMatches,
        &operation,
        &shape,
        &[],
        searching.scope,
    )
    .map(|contract| contract.failure)
}

/// The plans of a fold row, and the relation read from the project.
fn plans_of(plans: Plans) -> (Vec<Plan>, Option<Relation>) {
    let comment = Source::Relation(Relation::Comment);
    let tree = Source::Relation(Relation::SyntaxTree);
    let never = || select(at(0), vec![equal(1, "never")]);
    let mixed = || {
        vec![plan(vec![
            project(comment, &[3]),
            project(Source::Relation(Relation::SourceLocation), &[2]),
            step(Operation::Union {
                left: at(0),
                right: at(1),
            }),
            search(at(2), 0, "a", None),
        ])]
    };
    match plans {
        Plans::ProviderSpan => (
            vec![plan(vec![search(comment, 3, "a", None), never()])],
            None,
        ),
        Plans::Opened(opening) => {
            let first = match opening {
                Opening::Oversized => search(comment, 3, r"\w{1000}{1000}", None),
                Opening::MissingCapture => search(comment, 3, "(?P<x>a)", Some("y")),
                Opening::NamedCapture => search(comment, 3, "(?P<x>a)", Some("x")),
                Opening::RowError => step(Operation::SourceMatches {
                    from: comment,
                    span: ColumnIndex(3),
                    base_line: None,
                    pattern: LiteralText("a".into()),
                    capture: None,
                    capture_text: TextOutput::Appended,
                    capture_line: LineOutput::Omitted,
                }),
            };
            (vec![plan(vec![first, never()])], None)
        }
        // A base line: the start line of a source location.
        Plans::Numbered => (
            vec![plan(vec![
                step(Operation::SourceMatches {
                    from: Source::Relation(Relation::SourceLocation),
                    span: ColumnIndex(2),
                    base_line: Some(ColumnIndex(3)),
                    pattern: LiteralText("a".into()),
                    capture: None,
                    capture_text: TextOutput::Omitted,
                    capture_line: LineOutput::Omitted,
                }),
                select(at(0), vec![]),
            ])],
            None,
        ),
        Plans::Noqa => {
            // Steps 0-3: the heads, comment columns 0-4 then marker 5 and suffix span 6.
            let mut codes = vec![
                search(comment, 3, "#(?P<suffix>.*)$", Some("suffix")),
                search(comment, 3, "#:(?P<suffix>.*)$", Some("suffix")),
                project(at(0), &[0, 2, 5, 6]),
                project(at(1), &[0, 2, 5, 6]),
                step(Operation::Union {
                    left: at(2),
                    right: at(3),
                }),
            ];
            codes.extend([
                // The codes list over the suffix span: columns 0-3, then list 4 and codes span 5.
                search(at(4), 3, "^:(?P<codes>.*)$", Some("codes")),
                // A token over the codes span.
                search(at(5), 5, "(?P<code>[A-Z]+[0-9]+)", Some("code")),
                select(at(6), vec![]),
            ]);
            (vec![plan(codes)], None)
        }
        Plans::MixedShared => (mixed(), Some(Relation::SourceLocation)),
        Plans::MixedBoth => (mixed(), None),
        Plans::SelfJoin => (
            vec![plan(vec![
                select(tree, vec![]),
                step(Operation::Join {
                    left: at(0),
                    right: at(0),
                    left_on: vec![ColumnIndex(3)],
                    right_on: vec![ColumnIndex(3)],
                    output: vec![
                        JoinColumn {
                            side: Side::Left,
                            column: ColumnIndex(1),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(1),
                        },
                    ],
                    filter: vec![],
                    view: vec![],
                    reach: None,
                }),
                select(at(1), vec![equal(0, "x"), equal(1, "z")]),
            ])],
            None,
        ),
        Plans::JoinRight {
            view,
            reach,
            filter,
        } => (
            vec![plan(vec![
                select(comment, vec![]),
                step(Operation::Join {
                    left: at(0),
                    right: tree,
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                    output: vec![JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(0),
                    }],
                    filter: if filter { vec![equal(1, "y")] } else { vec![] },
                    view: if view { vec![ColumnIndex(2)] } else { vec![] },
                    reach: reach.then(|| ReachLookup {
                        source: ColumnIndex(0),
                        destination: ColumnIndex(1),
                        keys: vec![],
                        columns: vec![ColumnIndex(0), ColumnIndex(1)],
                    }),
                }),
                select(at(1), vec![equal(0, "x")]),
            ])],
            None,
        ),
        Plans::Union => (
            vec![plan(vec![
                select(tree, vec![equal(1, "x")]),
                select(tree, vec![equal(1, "y")]),
                step(Operation::Union {
                    left: at(0),
                    right: at(1),
                }),
                select(at(2), vec![equal(2, "z")]),
            ])],
            None,
        ),
        Plans::CrossFile => {
            let mut cross = plan(vec![
                select(tree, vec![equal(1, "x")]),
                select(at(0), vec![equal(1, "y")]),
            ]);
            cross.steps[0].scope = StepScope::CrossFile;
            cross.steps[1].scope = StepScope::CrossFile;
            (vec![cross], None)
        }
        Plans::UnknownColumn => (
            vec![plan(vec![
                select(tree, vec![equal(99, "x")]),
                select(at(0), vec![equal(1, "y")]),
            ])],
            None,
        ),
        Plans::FallibleUpstream => (
            vec![plan(vec![
                select(tree, vec![]),
                step(Operation::SpanBounds {
                    from: at(0),
                    span: ColumnIndex(4),
                }),
                select(at(1), vec![equal(1, "never")]),
            ])],
            None,
        ),
    }
}

/// A comparison as a row writes it.
fn compared(comparison: PlanComparison) -> Compared {
    Compared {
        test: comparison.test,
        column: if let Operand::Column(column) = comparison.left {
            Some(column.0)
        } else {
            None
        },
        text: if let Operand::Literal(QueryLiteral::Text(text)) = comparison.right {
            Some(Cow::Owned(text.0.into()))
        } else {
            None
        },
    }
}

/// Folds the plans: the plans that search a span fold with the request's slots declared and the
/// shared relation read from the project; the others fold as the file program does.
fn folded(plans: Plans) -> Skip {
    let (plans_folded, shared) = plans_of(plans);
    let presence = match plans {
        Plans::ProviderSpan
        | Plans::Opened(_)
        | Plans::Numbered
        | Plans::Noqa
        | Plans::MixedShared
        | Plans::MixedBoth => fold_sharing(&plans_folded, shared),
        Plans::SelfJoin
        | Plans::JoinRight { .. }
        | Plans::Union
        | Plans::CrossFile
        | Plans::UnknownColumn
        | Plans::FallibleUpstream => fold(&plans_folded),
    };
    let formula = last(&presence, &plans_folded);
    let shape = match &presence.table.formulas[formula.0] {
        PresenceFormula::Always => Shape::Always,
        PresenceFormula::Test(_) => Shape::Test,
        PresenceFormula::All(parts) => Shape::All(parts.len()),
        PresenceFormula::Any(parts) => Shape::Any(parts.len()),
    };
    let tests = conjuncts(&presence.table, formula)
        .into_iter()
        .filter(|(_, comparisons)| !comparisons.is_empty())
        .map(|(relation, comparisons)| {
            (
                relation,
                comparisons
                    .into_iter()
                    .map(compared)
                    .collect::<Cow<'static, [Compared]>>(),
            )
        })
        .collect();
    Skip {
        always: formula == ALWAYS,
        shape,
        tests,
        relied: relied(&presence)
            .into_iter()
            .map(|(relation, columns)| (relation, Cow::Owned(columns)))
            .collect(),
    }
}

fn gates(kind: OperationKind) -> Result<Gates, OperatingError> {
    let node = kinds(Relation::SyntaxChild);
    let shapes = [
        InputShape {
            kinds: node,
            tree: None,
        },
        InputShape {
            kinds: node,
            tree: None,
        },
        InputShape {
            kinds: node,
            tree: None,
        },
    ];
    let child = Source::Relation(Relation::SyntaxChild);
    let operation = if kind == OperationKind::Axis {
        Operation::Axis {
            from: child,
            edges: child,
            domain: child,
            direction: ryo_values::domain::plan_parts::types::Direction::Descendant,
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            domain_filter: vec![],
            domain_view: vec![],
            output: vec![],
        }
    } else {
        Operation::Antijoin {
            left: child,
            right: child,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        }
    };
    let kind = ryo_values::domain::plans::service::kind_of(&operation);
    let contract = prepare_for(kind, &operation, &shapes, &[], StepScope::PerFile)?;
    let (requires_all, mask) = match semantics(kind).presence {
        PresenceMask::AllOf(mask) => (true, mask.0),
        PresenceMask::AnyOf(_) | PresenceMask::Always => (false, 0),
    };
    let any = contract
        .empty_results
        .iter()
        .filter(|gate| gate.representation == EmptyRepresentation::Any);
    Ok(Gates {
        requires_all,
        any: any.clone().count(),
        outside_the_mask: any.filter(|gate| mask & (1 << gate.input.0) == 0).count(),
    })
}

fn regions() -> Regions {
    let plans = [plan(vec![select(
        Source::Relation(Relation::SyntaxTree),
        vec![equal(1, "x")],
    )])];
    let presence = fold(&plans);
    let node = super::types::FoldedNode(0);
    let region = |node, certified| super::types::RegionNode { node, certified };
    let table = for_regions(
        presence.clone(),
        [
            region(Some(node), super::types::Certification::Free),
            region(None, super::types::Certification::Free),
            region(Some(node), super::types::Certification::Certifying),
        ],
    );
    Regions {
        regions: table
            .regions
            .iter()
            .map(|formula| match *formula {
                formula if formula == presence.nodes[0] => Taken::Node,
                ALWAYS => Taken::Always,
                _ => Taken::Other,
            })
            .collect(),
        node_always: presence.nodes[0] == ALWAYS,
        reads: Cow::Owned(table.reads[0].to_vec()),
    }
}

/// An operation that raises an error from what reaches it, and the condition it raises it on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Raiser {
    /// On the values of its input rows.
    Rows(OperationKind),
    /// On a literal pattern that does not compile, when a row first reaches it.
    Pattern(OperationKind),
}

/// What can raise a fault: the values of rows that reach an operator, or something fixed before any row is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    /// Raised by these operations on rows of an input: each must declare the input fallible, through its prepared
    /// contract's failure mask or, for a literal pattern, through the fold.
    RowFallible(&'static [Raiser]),
    /// Raised by a comparison over a row; every literal comparison is inside its select's presence test, which holds
    /// when the comparison errs, so the select runs and reports it.
    Comparison,
    /// Fixed by the plan, the schema, a binding or the request, whatever the rows hold: such a node is `Always`, or its
    /// error is raised before rows are read.
    Fixed,
    /// The engine's own bookkeeping: raised the same whether a region runs or publishes empty.
    Engine,
    /// Named-input and semantic incompleteness: only downstream of a bound observation or a semantic renderer, both
    /// `Always` or row-fallible.
    Incomplete,
}

const DFS: &[Raiser] = &[Raiser::Rows(OperationKind::OrderedDfs)];
const ORDERED: &[Raiser] = &[Raiser::Rows(OperationKind::OrderedMatch)];
const SPAN: &[Raiser] = &[Raiser::Rows(OperationKind::SpanBetween)];
const SOURCE_SPAN: &[Raiser] = &[Raiser::Rows(OperationKind::SourceMatches)];
const BOUNDS: &[Raiser] = &[Raiser::Rows(OperationKind::SpanBounds)];
const RENDER: &[Raiser] = &[
    Raiser::Rows(OperationKind::RenderSyntax),
    Raiser::Rows(OperationKind::FitsRendered),
];
const TEXT_GATE: &[Raiser] = &[
    Raiser::Rows(OperationKind::Replace),
    Raiser::Rows(OperationKind::SourceMatches),
];
const CAPTURE: &[Raiser] = &[Raiser::Pattern(OperationKind::SourceMatches)];
const REGEX: &[Raiser] = &[
    Raiser::Rows(OperationKind::Select),
    Raiser::Rows(OperationKind::Join),
    Raiser::Rows(OperationKind::Axis),
    Raiser::Pattern(OperationKind::Select),
    Raiser::Pattern(OperationKind::Join),
    Raiser::Pattern(OperationKind::Axis),
    Raiser::Pattern(OperationKind::Replace),
    Raiser::Pattern(OperationKind::SourceMatches),
];

/// R6: every operating error and every portable operator fault, audited as row-dependent or not. The matches are
/// exhaustive, so a new variant cannot land without a verdict, and a row-dependent verdict names the operations that
/// raise it.
fn operating(error: &OperatingError) -> Fault {
    match error {
        OperatingError::DuplicateDfsOrder | OperatingError::DfsOrdinalOverflow => {
            Fault::RowFallible(DFS)
        }
        OperatingError::DuplicateOrder => Fault::RowFallible(ORDERED),
        OperatingError::InvalidSpan => Fault::RowFallible(SPAN),
        OperatingError::InvalidSourceMatchSpan => Fault::RowFallible(SOURCE_SPAN),
        OperatingError::InvalidSpanBounds => Fault::RowFallible(BOUNDS),
        OperatingError::SemanticRender(_) | OperatingError::SemanticBatchShape => {
            Fault::RowFallible(RENDER)
        }
        // A regular expression read from the row, or a literal one that does not compile.
        OperatingError::InvalidRegex { .. } => Fault::RowFallible(REGEX),
        OperatingError::WrongTest => Fault::Comparison,
        // Replace's text gate and SourceMatches' span gate; any other source is a schema check.
        OperatingError::ShapeMismatch => Fault::RowFallible(TEXT_GATE),
        OperatingError::Spelling(_)
        | OperatingError::ObservationConfiguration(_)
        | OperatingError::MissingInput { .. }
        | OperatingError::WrongOperation
        | OperatingError::Unbound
        | OperatingError::WrongBinding
        | OperatingError::Table(_) => Fault::Fixed,
        OperatingError::NotRun | OperatingError::NotRead => Fault::Engine,
        OperatingError::ObservationIncomplete { .. }
        | OperatingError::SemanticIncomplete { .. } => Fault::Incomplete,
    }
}
fn portable(fault: &ryo_operators::domain::contract::types::PortableOperatorFault) -> Fault {
    use ryo_operators::domain::contract::types::PortableOperatorFault::*;
    match fault {
        ShapeMismatch => Fault::RowFallible(TEXT_GATE),
        InvalidSpanBounds => Fault::RowFallible(BOUNDS),
        InvalidSpan => Fault::RowFallible(SPAN),
        InvalidSourceMatchSpan => Fault::RowFallible(SOURCE_SPAN),
        UnsupportedRender => Fault::RowFallible(RENDER),
        NoSuchColumn { .. } | WrongOperation | UnboundKey | WrongBinding | Unbound => Fault::Fixed,
        // A named capture the matcher lacks, raised when a row first reaches the search.
        MissingCapture => Fault::RowFallible(CAPTURE),
        Interrupted | NotRun => Fault::Engine,
        // A comparison's fault is the operating error it maps to, variant for variant.
        WrongTest => operating(&OperatingError::WrongTest),
        Pattern(_) => operating(&OperatingError::InvalidRegex {
            reason: String::new(),
        }),
    }
}

/// An operation of this kind in the condition its raiser names: a node column for Replace's text gate, a pattern read
/// from the row for a comparison, and the literal pattern `(` for a literal one.
fn raising(raiser: Raiser) -> Operation {
    let from = Source::Relation(Relation::Function);
    let comparisons = |pattern: Operand| {
        vec![PlanComparison {
            test: Test::Matches,
            left: Operand::Column(ColumnIndex(0)),
            right: pattern,
            origin: ComparisonOrigin::Unrecorded,
        }]
    };
    let (kind, pattern) = match raiser {
        Raiser::Rows(kind) => (kind, Operand::Column(ColumnIndex(1))),
        Raiser::Pattern(kind) => (
            kind,
            Operand::Literal(QueryLiteral::Text(LiteralText("(".into()))),
        ),
    };
    let literal = match &pattern {
        Operand::Literal(QueryLiteral::Text(text)) => text.clone(),
        _ => LiteralText("a".into()),
    };
    match kind {
        OperationKind::Select => Operation::Select {
            from,
            comparisons: comparisons(pattern),
            kept: vec![],
        },
        OperationKind::Join => Operation::Join {
            left: from,
            right: from,
            left_on: vec![],
            right_on: vec![],
            output: vec![],
            filter: comparisons(pattern),
            view: vec![],
            reach: None,
        },
        OperationKind::Axis => Operation::Axis {
            from,
            edges: from,
            domain: from,
            direction: ryo_values::domain::plan_parts::types::Direction::Descendant,
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            domain_filter: comparisons(pattern),
            domain_view: vec![],
            output: vec![],
        },
        OperationKind::Replace => Operation::Replace {
            from,
            // A node column has no text, so the text gate raises on its rows.
            text: ColumnIndex(if matches!(raiser, Raiser::Rows(_)) {
                0
            } else {
                1
            }),
            pattern: literal,
            replacement: LiteralText("b".into()),
        },
        // A per-file search over the comment span, so its contract declares `UnlessLocated`.
        OperationKind::SourceMatches => Operation::SourceMatches {
            from: Source::Relation(Relation::Comment),
            span: ColumnIndex(3),
            base_line: None,
            pattern: literal,
            capture: None,
            capture_text: TextOutput::Omitted,
            capture_line: LineOutput::Omitted,
        },
        OperationKind::OrderedDfs => Operation::OrderedDfs {
            from,
            edges: from,
            root: ColumnIndex(0),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            order: ColumnIndex(0),
            edge: ColumnIndex(0),
            root_keys: vec![],
            edge_keys: vec![],
        },
        OperationKind::OrderedMatch => Operation::OrderedMatch {
            left: from,
            right: from,
            left_on: vec![],
            right_on: vec![],
            left_order: ColumnIndex(0),
            right_order: ColumnIndex(0),
            output: vec![],
        },
        OperationKind::SpanBetween => Operation::SpanBetween {
            from,
            start: ColumnIndex(0),
            end: ColumnIndex(0),
        },
        OperationKind::SpanBounds => Operation::SpanBounds {
            from,
            span: ColumnIndex(0),
        },
        OperationKind::RenderSyntax => Operation::RenderSyntax {
            from,
            node: ColumnIndex(0),
            context: ryo_language::domain::semantics::types::RenderContext::AssignmentTarget,
        },
        OperationKind::FitsRendered => Operation::FitsRendered {
            from,
            texts: vec![],
            literals: vec![],
            span: ColumnIndex(0),
            max_line: ryo_values::domain::query_operators::types::MaxLine(88),
            tab_width: ryo_values::domain::query_operators::types::TabWidth(8),
        },
        other => panic!("no raising operation for {other:?}"),
    }
}

/// One fault the catalogue audits: an operating error or a portable operator fault.
#[derive(Clone, Copy, Debug)]
enum Raised<'a> {
    Operating(&'a OperatingError),
    Portable(&'a ryo_operators::domain::contract::types::PortableOperatorFault),
}

/// The proof that lets the fold drop an `UnlessLocated` failure term for a fault: the file's admission of the relied spans
/// through the search's own span gate (`validate_spans`), or the prepared slot's compile of the pattern and its named
/// capture (`relied`). The matches are exhaustive, so a new fault cannot land without saying whether a proof covers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocatedProof {
    SpanGate,
    MatcherSlot,
}
fn located_proof(raised: Raised<'_>) -> Option<LocatedProof> {
    use ryo_operators::domain::contract::types::PortableOperatorFault as P;
    match raised {
        Raised::Operating(
            OperatingError::InvalidSourceMatchSpan | OperatingError::ShapeMismatch,
        )
        | Raised::Portable(P::InvalidSourceMatchSpan | P::ShapeMismatch) => {
            Some(LocatedProof::SpanGate)
        }
        Raised::Operating(OperatingError::InvalidRegex { .. })
        | Raised::Portable(P::MissingCapture) => Some(LocatedProof::MatcherSlot),
        Raised::Operating(
            OperatingError::DuplicateDfsOrder
            | OperatingError::DfsOrdinalOverflow
            | OperatingError::DuplicateOrder
            | OperatingError::InvalidSpan
            | OperatingError::InvalidSpanBounds
            | OperatingError::SemanticRender(_)
            | OperatingError::SemanticBatchShape
            | OperatingError::WrongTest
            | OperatingError::Spelling(_)
            | OperatingError::ObservationConfiguration(_)
            | OperatingError::MissingInput { .. }
            | OperatingError::WrongOperation
            | OperatingError::Unbound
            | OperatingError::WrongBinding
            | OperatingError::Table(_)
            | OperatingError::NotRun
            | OperatingError::NotRead
            | OperatingError::ObservationIncomplete { .. }
            | OperatingError::SemanticIncomplete { .. },
        )
        | Raised::Portable(
            P::InvalidSpanBounds
            | P::InvalidSpan
            | P::UnsupportedRender
            | P::NoSuchColumn { .. }
            | P::WrongOperation
            | P::UnboundKey
            | P::WrongBinding
            | P::Unbound
            | P::Interrupted
            | P::NotRun,
        ) => None,
        Raised::Portable(P::Pattern(_)) => Some(LocatedProof::MatcherSlot),
        Raised::Portable(P::WrongTest) => None,
    }
}

/// Each operating error's place in the catalogue's list, by declaration order: a new variant must take a place here,
/// and then the list must hold an instance of it.
fn operating_position(error: &OperatingError) -> usize {
    match error {
        OperatingError::Spelling(_) => 0,
        OperatingError::ObservationConfiguration(_) => 1,
        OperatingError::ObservationIncomplete { .. } => 2,
        OperatingError::SemanticRender(_) => 3,
        OperatingError::SemanticBatchShape => 4,
        OperatingError::SemanticIncomplete { .. } => 5,
        OperatingError::DuplicateDfsOrder => 6,
        OperatingError::DfsOrdinalOverflow => 7,
        OperatingError::InvalidSpan => 8,
        OperatingError::InvalidSourceMatchSpan => 9,
        OperatingError::InvalidSpanBounds => 10,
        OperatingError::DuplicateOrder => 11,
        OperatingError::MissingInput { .. } => 12,
        OperatingError::WrongOperation => 13,
        OperatingError::NotRun => 14,
        OperatingError::NotRead => 15,
        OperatingError::Unbound => 16,
        OperatingError::WrongBinding => 17,
        OperatingError::WrongTest => 18,
        OperatingError::InvalidRegex { .. } => 19,
        OperatingError::ShapeMismatch => 20,
        OperatingError::Table(_) => 21,
    }
}
/// Each portable operator fault's place in the catalogue's list, by declaration order.
fn portable_position(
    fault: &ryo_operators::domain::contract::types::PortableOperatorFault,
) -> usize {
    use ryo_operators::domain::contract::types::PortableOperatorFault::*;
    match fault {
        Interrupted => 0,
        NoSuchColumn { .. } => 1,
        ShapeMismatch => 2,
        InvalidSpanBounds => 3,
        InvalidSpan => 4,
        InvalidSourceMatchSpan => 5,
        MissingCapture => 6,
        WrongOperation => 7,
        UnboundKey => 8,
        UnsupportedRender => 9,
        WrongBinding => 10,
        Unbound => 11,
        NotRun => 12,
        WrongTest => 13,
        Pattern(_) => 14,
    }
}

/// Whether the raiser declares the input it raises this fault on: the prepared contract's failure mask, or the fold's
/// check of a literal pattern. `UnlessLocated` declares a fault only when a proof the fold requires before dropping the
/// term covers it.
fn declared(raiser: Raiser, raised: Raised<'_>) -> bool {
    let operation = raising(raiser);
    let kind = ryo_values::domain::plans::service::kind_of(&operation);
    let relation = match kind {
        OperationKind::SourceMatches => Relation::Comment,
        _ => Relation::Function,
    };
    let shapes = [InputShape {
        kinds: kinds(relation),
        tree: None,
    }; 3];
    let failure = prepare_for(kind, &operation, &shapes, &[], StepScope::PerFile)
        .map(|contract| contract.failure);
    let rows = matches!(failure, Ok(RowFailure::OnRows(mask)) if mask != InputMask(0))
        || (matches!(failure, Ok(RowFailure::UnlessLocated(_))) && located_proof(raised).is_some());
    match raiser {
        Raiser::Rows(_) => rows,
        Raiser::Pattern(_) => {
            rows || super::narrowing::service::invalid_patterns(
                &operation,
                ryo_operators::domain::patterns::types::PatternMatcher(
                    &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
                ),
                &ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers::default(
                ),
            )
            .is_some()
        }
    }
}

fn verdicts() -> Verdicts {
    use ryo_operators::domain::contract::types::PortableOperatorFault;
    let reason = || String::new();
    // One instance of every variant, in declaration order: `operating_position` and `portable_position` are exhaustive,
    // and each list must map onto 0..len, so a new variant cannot be audited by omission.
    let errors = [
        OperatingError::Spelling(
            crate::domain::spelling::errors::SpellingError::EntityInTwoSources {
                first: reason(),
                second: reason(),
            },
        ),
        OperatingError::ObservationConfiguration(
            crate::domain::rule_intent::errors::RuleIntentError::TooManyOutputs,
        ),
        OperatingError::ObservationIncomplete { reason: reason() },
        OperatingError::SemanticRender(
            ryo_language::domain::semantics::errors::SemanticsError::StaleNode {
                node: ryo_relations::domain::relations::types::NodeId(0),
            },
        ),
        OperatingError::SemanticBatchShape,
        OperatingError::SemanticIncomplete { reason: reason() },
        OperatingError::DuplicateDfsOrder,
        OperatingError::DfsOrdinalOverflow,
        OperatingError::InvalidSpan,
        OperatingError::InvalidSourceMatchSpan,
        OperatingError::InvalidSpanBounds,
        OperatingError::DuplicateOrder,
        OperatingError::MissingInput { position: 0 },
        OperatingError::WrongOperation,
        OperatingError::NotRun,
        OperatingError::NotRead,
        OperatingError::Unbound,
        OperatingError::WrongBinding,
        OperatingError::WrongTest,
        OperatingError::InvalidRegex { reason: reason() },
        OperatingError::ShapeMismatch,
        OperatingError::Table(ryo_values::domain::rows::errors::RowsError::NoSuchColumn {
            index: 0,
        }),
    ];
    let faults = [
        PortableOperatorFault::Interrupted,
        PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(0),
        },
        PortableOperatorFault::ShapeMismatch,
        PortableOperatorFault::InvalidSpanBounds,
        PortableOperatorFault::InvalidSpan,
        PortableOperatorFault::InvalidSourceMatchSpan,
        PortableOperatorFault::MissingCapture,
        PortableOperatorFault::WrongOperation,
        PortableOperatorFault::UnboundKey,
        PortableOperatorFault::UnsupportedRender,
        PortableOperatorFault::WrongBinding,
        PortableOperatorFault::Unbound,
        PortableOperatorFault::NotRun,
    ];
    let errors_in_order = errors.iter().map(operating_position).eq(0..errors.len());
    let faults_in_order = faults.iter().map(portable_position).eq(0..faults.len());
    let verdicts = errors
        .iter()
        .map(|error| (Raised::Operating(error), operating(error)))
        .chain(
            faults
                .iter()
                .map(|fault| (Raised::Portable(fault), portable(fault))),
        );
    let mut undeclared = Vec::new();
    let mut searched = Vec::new();
    let mut proven = Vec::new();
    for (raised, verdict) in verdicts {
        if let Fault::RowFallible(raisers) = verdict {
            for raiser in raisers {
                if !declared(*raiser, raised) {
                    undeclared.push(Cow::Owned(format!("{raiser:?} raises {raised:?}")));
                }
            }
            if raisers.iter().any(|raiser| {
                matches!(
                    raiser,
                    Raiser::Rows(OperationKind::SourceMatches)
                        | Raiser::Pattern(OperationKind::SourceMatches)
                )
            }) {
                searched.push(Cow::Owned(format!("{raised:?}")));
            }
        }
        if located_proof(raised).is_some() {
            proven.push(Cow::Owned(format!("{raised:?}")));
        }
    }
    let oversized = Operation::Select {
        from: Source::Relation(Relation::Function),
        comparisons: vec![PlanComparison {
            test: Test::Matches,
            left: Operand::Column(ColumnIndex(1)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(r"\w{1000}{1000}".into()))),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: vec![],
    };
    Verdicts {
        errors_in_order,
        faults_in_order,
        undeclared: Cow::Owned(undeclared),
        searched: Cow::Owned(searched),
        proven: Cow::Owned(proven),
        oversized: super::narrowing::service::invalid_patterns(
            &oversized,
            ryo_operators::domain::patterns::types::PatternMatcher(
                &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
            ),
            &ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers::default(),
        ),
        not_run: operating(&OperatingError::NotRun),
        missing_capture: portable(&PortableOperatorFault::MissingCapture),
    }
}

/// An equality of a column and a text, as a row writes it.
const fn eq(column: u32, text: &'static str) -> Compared {
    Compared {
        test: Test::Equal,
        column: Some(column),
        text: Some(Cow::Borrowed(text)),
    }
}

/// The skip formula a fold answers.
const fn skip(
    always: bool,
    shape: Shape,
    tests: &'static [Tested],
    relied: &'static [(Relation, Cow<'static, [u32]>)],
) -> Answer {
    Answer::Skip(Skip {
        always,
        shape,
        tests: Cow::Borrowed(tests),
        relied: Cow::Borrowed(relied),
    })
}

/// The comment span column 3, relied on.
const COMMENT_SPAN: &[(Relation, Cow<'static, [u32]>)] =
    &[(Relation::Comment, Cow::Borrowed(&[3]))];

/// A source search over column 3, per file, with no base line, capture or appended text.
const fn searching(span: u32) -> Searching {
    Searching {
        span,
        base_line: None,
        capture: None,
        text: TextOutput::Omitted,
        scope: StepScope::PerFile,
    }
}

/// The faults a source search raises on rows, each covered by a located proof.
const SEARCHED: &[Cow<'static, str>] = &[
    Cow::Borrowed("Operating(InvalidSourceMatchSpan)"),
    Cow::Borrowed("Operating(InvalidRegex { reason: \"\" })"),
    Cow::Borrowed("Operating(ShapeMismatch)"),
    Cow::Borrowed("Portable(ShapeMismatch)"),
    Cow::Borrowed("Portable(InvalidSourceMatchSpan)"),
    Cow::Borrowed("Portable(MissingCapture)"),
];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_operation_declares_the_inputs_its_rows_need"),
        word: Word::Succeeds,
        input: || Asked::Declarations,
        expect: Expect::Value(Answer::Declarations(Declarations {
            kinds: 26,
            differing: Cow::Borrowed(&[]),
        })),
    },
    Case {
        name: CaseName(
            "replace_can_err_on_rows_unless_a_per_file_step_reads_a_symbol_or_file_column",
        ),
        word: Word::Succeeds,
        input: || Asked::Replace(1, StepScope::PerFile),
        expect: Expect::Value(Answer::Failure(RowFailure::Never)),
    },
    Case {
        name: CaseName("a_per_file_replace_of_a_file_column_cannot_err"),
        word: Word::Succeeds,
        input: || Asked::Replace(2, StepScope::PerFile),
        expect: Expect::Value(Answer::Failure(RowFailure::Never)),
    },
    Case {
        name: CaseName("a_replace_of_a_node_column_can_err"),
        word: Word::Succeeds,
        input: || Asked::Replace(0, StepScope::PerFile),
        expect: Expect::Value(Answer::Failure(RowFailure::OnRows(InputMask(1)))),
    },
    Case {
        name: CaseName("a_cross_file_replace_can_err"),
        word: Word::Succeeds,
        input: || Asked::Replace(1, StepScope::CrossFile),
        expect: Expect::Value(Answer::Failure(RowFailure::OnRows(InputMask(1)))),
    },
    Case {
        name: CaseName("a_replace_of_a_column_the_input_lacks_can_err"),
        word: Word::Succeeds,
        input: || Asked::Replace(9, StepScope::PerFile),
        expect: Expect::Value(Answer::Failure(RowFailure::OnRows(InputMask(1)))),
    },
    Case {
        name: CaseName("the_row_fallible_operators_name_the_inputs_whose_rows_can_err"),
        word: Word::Succeeds,
        input: || Asked::Fallible(OperationKind::Select),
        expect: Expect::Value(Answer::Failure(RowFailure::Never)),
    },
    Case {
        name: CaseName("an_ordered_match_can_err_on_both_inputs"),
        word: Word::Succeeds,
        input: || Asked::Fallible(OperationKind::OrderedMatch),
        expect: Expect::Value(Answer::Failure(RowFailure::OnRows(InputMask(3)))),
    },
    Case {
        name: CaseName("span_bounds_can_err_on_its_input"),
        word: Word::Succeeds,
        input: || Asked::Fallible(OperationKind::SpanBounds),
        expect: Expect::Value(Answer::Failure(RowFailure::OnRows(InputMask(1)))),
    },
    Case {
        name: CaseName("every_always_gated_empty_result_lies_inside_the_presence_mask"),
        word: Word::Succeeds,
        input: || Asked::Gates(OperationKind::Antijoin),
        expect: Expect::Value(Answer::Gates(Gates {
            requires_all: true,
            any: 1,
            outside_the_mask: 0,
        })),
    },
    Case {
        name: CaseName("an_axis_gates_inside_its_presence_mask"),
        word: Word::Succeeds,
        input: || Asked::Gates(OperationKind::Axis),
        expect: Expect::Value(Answer::Gates(Gates {
            requires_all: true,
            any: 1,
            outside_the_mask: 0,
        })),
    },
    Case {
        name: CaseName("a_self_join_tests_each_side_on_its_own"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::SelfJoin),
        expect: Expect::Value(skip(
            false,
            Shape::All(3),
            const {
                &[
                    (Relation::SyntaxTree, Cow::Borrowed(const { &[eq(1, "x")] })),
                    (Relation::SyntaxTree, Cow::Borrowed(const { &[eq(1, "z")] })),
                ]
            },
            &[],
        )),
    },
    Case {
        name: CaseName("join_right_columns_resolve_through_the_schema_functions"),
        word: Word::Succeeds,
        input: || {
            Asked::Fold(Plans::JoinRight {
                view: true,
                reach: false,
                filter: false,
            })
        },
        expect: Expect::Value(skip(
            false,
            Shape::All(2),
            const { &[(Relation::SyntaxTree, Cow::Borrowed(const { &[eq(2, "x")] }))] },
            &[],
        )),
    },
    Case {
        name: CaseName("a_join_with_no_view_reads_the_right_column_itself"),
        word: Word::Succeeds,
        input: || {
            Asked::Fold(Plans::JoinRight {
                view: false,
                reach: false,
                filter: false,
            })
        },
        expect: Expect::Value(skip(
            false,
            Shape::All(2),
            const { &[(Relation::SyntaxTree, Cow::Borrowed(const { &[eq(0, "x")] }))] },
            &[],
        )),
    },
    Case {
        name: CaseName("a_join_filter_tests_the_raw_right_input"),
        word: Word::Succeeds,
        input: || {
            Asked::Fold(Plans::JoinRight {
                view: false,
                reach: false,
                filter: true,
            })
        },
        expect: Expect::Value(skip(
            false,
            Shape::All(2),
            const {
                &[
                    (Relation::SyntaxTree, Cow::Borrowed(const { &[eq(1, "y")] })),
                    (Relation::SyntaxTree, Cow::Borrowed(const { &[eq(0, "x")] })),
                ]
            },
            &[],
        )),
    },
    Case {
        name: CaseName("a_reach_join_right_side_has_no_origin_and_its_filter_drops_out"),
        word: Word::Succeeds,
        input: || {
            Asked::Fold(Plans::JoinRight {
                view: false,
                reach: true,
                filter: true,
            })
        },
        expect: Expect::Value(skip(false, Shape::All(2), &[], &[])),
    },
    Case {
        name: CaseName("a_union_is_any_of_its_branches_and_resets_origins"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::Union),
        expect: Expect::Value(skip(false, Shape::Any(2), &[], &[])),
    },
    Case {
        name: CaseName("unconditional_nodes_and_their_consumers_are_always"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::CrossFile),
        expect: Expect::Value(skip(true, Shape::Always, &[], &[])),
    },
    Case {
        name: CaseName("a_step_over_an_invalid_schema_and_its_consumers_are_always"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::UnknownColumn),
        expect: Expect::Value(skip(true, Shape::Always, &[], &[])),
    },
    Case {
        name: CaseName("a_fallible_operator_upstream_holds_the_skip_formula_open"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::FallibleUpstream),
        expect: Expect::Value(skip(false, Shape::Any(2), &[], &[])),
    },
    Case {
        name: CaseName("the_region_table_takes_each_regions_node"),
        word: Word::Succeeds,
        input: || Asked::Regions,
        expect: Expect::Value(Answer::Regions(Regions {
            regions: Cow::Borrowed(&[Taken::Node, Taken::Always, Taken::Always]),
            node_always: false,
            reads: Cow::Borrowed(&[Relation::SyntaxTree]),
        })),
    },
    Case {
        name: CaseName("every_fault_has_a_verdict"),
        word: Word::Succeeds,
        input: || Asked::Verdicts,
        expect: Expect::Value(Answer::Verdicts(Verdicts {
            errors_in_order: true,
            faults_in_order: true,
            undeclared: Cow::Borrowed(&[]),
            searched: Cow::Borrowed(SEARCHED),
            proven: Cow::Borrowed(SEARCHED),
            oversized: Some(InputMask(1)),
            not_run: Fault::Engine,
            missing_capture: Fault::RowFallible(CAPTURE),
        })),
    },
    Case {
        name: CaseName(
            "source_matches_can_err_on_rows_unless_per_file_over_a_span_without_base_line_or_row_error",
        ),
        word: Word::Succeeds,
        input: || {
            Asked::Search(Searching {
                capture: Some("x"),
                text: TextOutput::Appended,
                ..searching(3)
            })
        },
        expect: Expect::Value(Answer::Failure(RowFailure::UnlessLocated(ColumnIndex(3)))),
    },
    Case {
        name: CaseName("a_cross_file_search_can_err"),
        word: Word::Succeeds,
        input: || {
            Asked::Search(Searching {
                scope: StepScope::CrossFile,
                ..searching(3)
            })
        },
        expect: Expect::Value(Answer::Failure(RowFailure::OnRows(InputMask(1)))),
    },
    Case {
        name: CaseName("a_search_from_a_base_line_read_from_the_row_can_err"),
        word: Word::Succeeds,
        input: || {
            Asked::Search(Searching {
                base_line: Some(0),
                ..searching(3)
            })
        },
        expect: Expect::Value(Answer::Failure(RowFailure::OnRows(InputMask(1)))),
    },
    Case {
        name: CaseName("a_search_appending_text_with_no_capture_can_err"),
        word: Word::Succeeds,
        input: || {
            Asked::Search(Searching {
                text: TextOutput::Appended,
                ..searching(3)
            })
        },
        expect: Expect::Value(Answer::Failure(RowFailure::OnRows(InputMask(1)))),
    },
    Case {
        name: CaseName("a_search_over_a_column_that_is_no_span_can_err"),
        word: Word::Succeeds,
        input: || Asked::Search(searching(1)),
        expect: Expect::Value(Answer::Failure(RowFailure::OnRows(InputMask(1)))),
    },
    Case {
        name: CaseName("a_search_over_a_provider_span_adds_no_failure_term"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::ProviderSpan),
        expect: Expect::Value(skip(
            false,
            Shape::All(2),
            const {
                &[(
                    Relation::Comment,
                    Cow::Borrowed(const { &[eq(1, "never")] }),
                )]
            },
            COMMENT_SPAN,
        )),
    },
    Case {
        name: CaseName("a_search_that_can_still_fail_keeps_its_failure_term"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::Opened(Opening::Oversized)),
        expect: Expect::Value(skip(false, Shape::Any(2), &[], &[])),
    },
    Case {
        name: CaseName("a_search_missing_its_named_capture_keeps_its_failure_term"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::Opened(Opening::MissingCapture)),
        expect: Expect::Value(skip(false, Shape::Any(2), &[], &[])),
    },
    Case {
        name: CaseName("a_search_with_its_named_capture_adds_no_failure_term"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::Opened(Opening::NamedCapture)),
        expect: Expect::Value(skip(
            false,
            Shape::All(2),
            const {
                &[(
                    Relation::Comment,
                    Cow::Borrowed(const { &[eq(1, "never")] }),
                )]
            },
            COMMENT_SPAN,
        )),
    },
    Case {
        name: CaseName("a_search_from_a_base_line_is_conditional_and_relies_on_no_span"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::Numbered),
        expect: Expect::Value(skip(false, Shape::Test, &[], &[])),
    },
    Case {
        name: CaseName("a_search_with_a_row_error_keeps_its_failure_term"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::Opened(Opening::RowError)),
        expect: Expect::Value(skip(false, Shape::Any(2), &[], &[])),
    },
    Case {
        name: CaseName("located_spans_pass_a_union_of_located_sides"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::Noqa),
        expect: Expect::Value(skip(false, Shape::Test, &[], COMMENT_SPAN)),
    },
    Case {
        name: CaseName("a_union_with_a_side_read_from_the_project_locates_no_span"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::MixedShared),
        expect: Expect::Value(skip(true, Shape::Always, &[], &[])),
    },
    Case {
        name: CaseName("a_union_of_file_sides_relies_on_both_spans"),
        word: Word::Succeeds,
        input: || Asked::Fold(Plans::MixedBoth),
        expect: Expect::Value(skip(
            false,
            Shape::Any(2),
            &[],
            &[
                (Relation::Comment, Cow::Borrowed(&[3])),
                (Relation::SourceLocation, Cow::Borrowed(&[2])),
            ],
        )),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
