//! A region the recogniser did not resolve keeps its registered operators, equivalent drafted regions merge as the linear scan
//! would in linear cost, and demand compilation preserves declared order.
use super::fixtures::{Cost, Merge, axis, merge_beside_the_linear_scan, merge_cost};
use super::{
    service::ordered_existence,
    types::{DemandAction, Route, RouteKind, RowGate, demand_program::value::DemandProgram},
};
use crate::domain::{
    operation_schema::types::InputShape, operator::contract::service::prepare_for,
    operator::contract::types::DemandOrder,
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Input as PlanInput, StepScope},
        plans::types::Operation,
    },
};
use std::borrow::Cow;
/// What a case asks.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The demand program of an axis contract whose demand order is replaced by this order, over this
    /// many inputs.
    Demand(DemandOrder, usize),
    /// The region merge of 40 rules in 7 variants, beside the linear scan it replaced.
    MergeBesideLinearScan,
    /// The comparisons of the region merge of 50, 100, 200 and 400 rules.
    MergeCost,
    /// The route of a region the recogniser did not resolve.
    UnresolvedRegion,
    /// The row gate of a replace over a symbol column, for a name with no text.
    NameWithoutText,
    /// The place of every original step of three rules, beside the logical lookup.
    StepOrigins,
}

/// The demand order the axis contract prepared, and the inputs the compiled program admits, in order.
#[derive(Debug, PartialEq)]
struct Demand {
    prepared: DemandOrder,
    admits: Cow<'static, [usize]>,
}

/// The route an unresolved region gets, and whether it has an ordered existence.
#[derive(Debug, PartialEq)]
struct Unresolved {
    route: RouteKind,
    ordered_existence: bool,
}

/// The original steps of the rules, how many have a place, and how many places name what the lookup finds.
#[derive(Debug, PartialEq)]
struct Origins {
    steps: usize,
    placed: usize,
    as_looked_up: usize,
}

/// What a case answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Demand(Demand),
    Merge(Merge),
    Cost(Cost),
    Unresolved(Unresolved),
    Gate(RowGate),
    Origins(Origins),
}

type Input = Asked;
type Value = Answer;
type Failure = crate::domain::operating::errors::OperatingError;

const fn demand(admits: &'static [usize]) -> Answer {
    Answer::Demand(Demand {
        prepared: DemandOrder::LeftThenRight,
        admits: Cow::Borrowed(admits),
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_inputs"),
        word: Word::Succeeds,
        input: || Asked::Demand(DemandOrder::None, 0),
        expect: Expect::Value(demand(&[])),
    },
    Case {
        name: CaseName("unary"),
        word: Word::Succeeds,
        input: || Asked::Demand(DemandOrder::Unary, 1),
        expect: Expect::Value(demand(&[0])),
    },
    Case {
        name: CaseName("empty_ordered"),
        word: Word::Succeeds,
        input: || Asked::Demand(DemandOrder::LeftThenRight, 0),
        expect: Expect::Value(demand(&[])),
    },
    Case {
        name: CaseName("one_ordered"),
        word: Word::Succeeds,
        input: || Asked::Demand(DemandOrder::LeftThenRight, 1),
        expect: Expect::Value(demand(&[0])),
    },
    Case {
        name: CaseName("two_ordered"),
        word: Word::Succeeds,
        input: || Asked::Demand(DemandOrder::LeftThenRight, 2),
        expect: Expect::Value(demand(&[0, 1])),
    },
    Case {
        name: CaseName("three_ordered"),
        word: Word::Succeeds,
        input: || Asked::Demand(DemandOrder::LeftThenRight, 3),
        expect: Expect::Value(demand(&[0, 1, 2])),
    },
    Case {
        name: CaseName("merge_matches_the_linear_scan"),
        word: Word::Succeeds,
        input: || Asked::MergeBesideLinearScan,
        expect: Expect::Value(Answer::Merge(Merge {
            kept: 25,
            kept_as_the_scan: true,
            drafts: 80,
            drafts_left: 30,
            drafts_left_as_the_scan: true,
            mapping_as_the_scan: true,
            representatives_as_the_scan: true,
            readers_as_the_scan: true,
        })),
    },
    Case {
        name: CaseName("regions_merge_cost_is_linear_in_rule_count"),
        word: Word::Succeeds,
        input: || Asked::MergeCost,
        expect: Expect::Value(Answer::Cost(Cost {
            comparisons: Cow::Borrowed(&[48, 98, 198, 398]),
            linear: true,
        })),
    },
    Case {
        name: CaseName("a_name_without_text_is_refused_by_the_replace_text_gate"),
        word: Word::Refuses,
        input: || Asked::NameWithoutText,
        expect: Expect::Error(crate::domain::operating::errors::OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("step_origin_place_equals_the_logical_lookup"),
        word: Word::Succeeds,
        input: || Asked::StepOrigins,
        expect: Expect::Value(Answer::Origins(Origins {
            steps: 6,
            placed: 6,
            as_looked_up: 6,
        })),
    },
    Case {
        name: CaseName("an_unresolved_region_keeps_its_registered_operators"),
        word: Word::Succeeds,
        input: || Asked::UnresolvedRegion,
        expect: Expect::Value(Answer::Unresolved(Unresolved {
            route: RouteKind::Generic,
            ordered_existence: false,
        })),
    },
];
fn subject(asked: Input) -> Result<Value, Failure> {
    match asked {
        Asked::Demand(order, count) => demand_program(order, count).map(Answer::Demand),
        Asked::MergeBesideLinearScan => Ok(Answer::Merge(merge_beside_the_linear_scan())),
        Asked::MergeCost => Ok(Answer::Cost(merge_cost())),
        Asked::NameWithoutText => a_name_without_text().map(Answer::Gate),
        Asked::StepOrigins => step_origins().map(Answer::Origins),
        Asked::UnresolvedRegion => {
            let route = Route::default();
            Ok(Answer::Unresolved(Unresolved {
                route: match route {
                    Route::Generic => RouteKind::Generic,
                    Route::OrderedExistence(_) => RouteKind::OrderedExistence,
                },
                ordered_existence: ordered_existence(&route).is_some(),
            }))
        }
    }
}
fn demand_program(order: DemandOrder, count: usize) -> Result<Demand, Failure> {
    let operation = axis(0);
    let inputs = [
        InputShape {
            kinds: &[CellKind::Node],
            tree: None,
        },
        InputShape {
            kinds: &[CellKind::Node, CellKind::Node],
            tree: None,
        },
        InputShape {
            kinds: &[CellKind::Node],
            tree: None,
        },
    ];
    let mut contract = prepare_for(
        OperationKind::Axis,
        &operation,
        &inputs,
        &[],
        StepScope::PerFile,
    )?;
    let prepared = contract.semantics.demand;
    contract.semantics.demand = order;
    let program = DemandProgram::compiled(
        contract.semantics,
        ryo_values::domain::plan_parts::types::InputCount(count),
    );
    let admits = program
        .actions()
        .iter()
        .map(|action| match action {
            DemandAction::Admit(input) => input.0,
        })
        .collect();
    Ok(Demand { prepared, admits })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The boundary the Replace declaration rests on: a name that left its file and has no text fails the text gate, so only
/// a path where no such name can arrive may declare Replace over a Symbol column infallible.
fn a_name_without_text() -> Result<RowGate, Failure> {
    let kinds = [CellKind::Symbol];
    let contract = prepare_for(
        OperationKind::Replace,
        &Operation::Replace {
            from: PlanInput::Relation(Relation::Comment),
            text: ColumnIndex(0),
            pattern: ryo_values::domain::plan_parts::types::LiteralText("a".into()),
            replacement: ryo_values::domain::plan_parts::types::LiteralText("b".into()),
        },
        &[InputShape {
            kinds: &kinds,
            tree: None,
        }],
        &[],
        StepScope::PerFile,
    )?;
    let symbols = ryo_language::domain::symbols::types::Symbols::default();
    let texts = ryo_operators::domain::table_texts::types::Texts {
        symbols: &symbols,
        source: ryo_values::domain::files::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    let row = [ryo_relations::domain::relations::types::Value::Name(
        ryo_relations::domain::relations::types::Name {
            hash: ryo_relations::domain::relations::types::TextHash(7),
            place: ryo_relations::domain::relations::types::PlaceNumber(0),
        },
    )];
    crate::domain::interpreted_program::kernel::service::row_admissible(&contract, &row, &texts)
}

/// Each original step's place, filled when the rule programs are built, still names the instruction today's lookup finds on the
/// finished program: its logical instruction's region, and the instruction's place there.
fn step_origins() -> Result<Origins, Failure> {
    use crate::domain::source_partitions::types::source_partitions::value::SourcePartitions;
    use ryo_values::domain::{
        plan_parts::types::{Input, StepId},
        plans::types::{Emit, Plan, PlanStep},
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        runs::types::{RuleApplicability, RuleId},
    };
    let step = |operation| PlanStep {
        operation,
        scope: StepScope::PerFile,
    };
    let select = |from| {
        step(Operation::Select {
            from,
            comparisons: vec![],
            kept: vec![],
        })
    };
    let plan = |steps: Vec<PlanStep>| Plan {
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
    };
    let plans = [
        plan(vec![
            select(PlanInput::Relation(Relation::Function)),
            select(Input::Step(StepId(0))),
        ]),
        plan(vec![
            select(PlanInput::Relation(Relation::Function)),
            select(Input::Step(StepId(0))),
            select(Input::Step(StepId(1))),
        ]),
        plan(vec![select(PlanInput::Relation(Relation::Comment))]),
    ];
    let rules = (0..plans.len())
        .map(|rule| RuleSpec {
            id: RuleId(format!("rule-{rule}").into()),
            query: QueryBundle {
                root: QueryDocumentSource {
                    id: QuerySourceId(format!("rule-{rule}").into()),
                    text: "".into(),
                },
                modules: Default::default(),
            },
            applicability: RuleApplicability::Always,
            given: vec![],
            observations: vec![],
            spelled: RuleParameters("".into()),
            include: vec![],
            exclude: vec![],
        })
        .collect::<Vec<_>>();
    let bindings = vec![vec![]; plans.len()];
    let operators = crate::domain::file_program::service::operators::service::prepare(
        &plans,
        &bindings,
        &[],
        &ryo_relations::domain::relations::service::kinds,
        &mut |_| Ok(()),
    )?;
    let borrowed = plans.iter().map(Ok).collect::<Vec<_>>();
    let partitions = SourcePartitions::new(
        &rules,
        &borrowed,
        &bindings,
        &[],
        &vec![vec![]; plans.len()],
    )?;
    let graph = crate::domain::file_program::service::compile_prepared(&plans, &operators);
    let program = super::service::compile(
        &plans,
        graph,
        &partitions,
        &operators,
        ryo_operators::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
    )?;
    let mut origins = Origins {
        steps: 0,
        placed: 0,
        as_looked_up: 0,
    };
    for rule in program.rules() {
        for origin in rule.original_steps.iter().flatten() {
            let lookup = program
                .logical_regions()
                .get(origin.logical.0)
                .and_then(|region| {
                    program.regions().get(region.0).and_then(|code| {
                        code.instructions()
                            .iter()
                            .position(|instruction| instruction.logical() == origin.logical)
                            .map(|at| super::types::StepSite {
                                region: *region,
                                instruction: super::types::InstructionPlace(at),
                            })
                    })
                });
            origins.steps += 1;
            origins.placed += usize::from(origin.place.is_some());
            origins.as_looked_up += usize::from(origin.place.is_some() && origin.place == lookup);
        }
    }
    Ok(origins)
}
