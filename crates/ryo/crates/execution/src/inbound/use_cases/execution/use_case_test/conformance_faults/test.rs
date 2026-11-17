//! Each operation's failure declaration rests on its kernel: every mutation of a kind's fixture operation whose schema passes
//! and whose kernel faults over the fixture's populated inputs is one the activation plan reads as able to fault (its own
//! `failure()` mask names an input, or the node is live), so no reader of it is skipped where it would have raised the fault.
use super::super::super::fixtures as owner_fixtures;
use super::super::conformance::fixtures::deliver;
use super::super::conformance::test::{author, schema};
use super::fixtures::{Mutation, mutations};
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::activation::types::NodeRequirement;
use ryo_execution_domain::domain::executable_program::service::{
    activation, assemble, logical, rule,
};
use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
use ryo_execution_domain::domain::executable_program::types::{
    RuleBindings, RuleInput, RuleObservations,
};
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::expectations::SOURCE;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::fixture;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::values::KnownSyntax;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;
use std::sync::Arc;

/// Whether the activation plan reads a node as able to fault on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Declared {
    /// Its `failure()` mask names an input, or it is live.
    Fallible,
    /// Neither: a reader of it may be skipped where it has rows.
    Never,
}

/// The node the authored plan of `mutation` emits, as the activation plan reads it; `None` when the logical program refuses
/// the step, which then answers its refusal by running.
fn declared(
    authored: &ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs::AuthoredRows,
    mutation: &Mutation,
) -> Option<Declared> {
    let plan = Arc::new(authored.plan.clone());
    let rules = [RuleInput {
        plan: &plan,
        bindings: RuleBindings(&mutation.bindings),
        observations: RuleObservations(&plan.observation_inputs),
    }];
    let mut inputs = ProgramInputs::hosting(
        authored
            .hosted
            .iter()
            .map(|relation| {
                relation
                    .columns
                    .iter()
                    .map(|column| ryo_values::domain::hosted::service::kind_of(column.kind))
                    .collect()
            })
            .collect(),
    );
    let program = assemble(logical(&rules, &mut inputs).ok()?, Default::default());
    let node = rule(&program, RuleIndex(0)).ok()?.emit.clone().ok()?;
    Some(
        match activation(&program)
            .nodes
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))?
        {
            NodeRequirement::Live => Declared::Fallible,
            NodeRequirement::Kinds { fallible, .. } | NodeRequirement::Inputs { fallible, .. }
                if fallible.0 != 0 =>
            {
                Declared::Fallible
            }
            NodeRequirement::Kinds { .. } | NodeRequirement::Inputs { .. } => Declared::Never,
        },
    )
}

/// The faults one kind's mutations raise over populated inputs: how many raise one, and each that the activation plan reads as
/// unable to fault, with its fault.
#[derive(Debug, PartialEq, Eq)]
struct Coverage {
    faulting: usize,
    uncovered: Cow<'static, [Cow<'static, str>]>,
}

type Input = OperationKind;
type Value = Coverage;
type Failure = OperatingError;

fn subject(kind: Input) -> Result<Value, Failure> {
    let semantic = KnownSyntax;
    let base = fixture(kind);
    let mut faulting = 0;
    let mut uncovered = Vec::new();
    for mutation in mutations(&base.operation, &base.bindings) {
        let mut case = fixture(kind);
        let checked = schema(&mutation.operation, &case.inputs, &mutation.bindings);
        if checked.is_err() {
            // A refused schema is answered before any row; its node is live.
            continue;
        }
        let authored = author(
            &mutation.operation,
            &case.inputs,
            &mutation.bindings,
            &mut case.symbols,
        );
        let declaration = declared(&authored, &mutation);
        let Ok(observed) = deliver(
            authored,
            checked,
            &mutation.bindings,
            SOURCE,
            &mut case.symbols,
            Some(&semantic),
            Arc::new(owner_fixtures::operators()),
        ) else {
            // The request was refused while it was prepared: no row was read.
            continue;
        };
        if let Err(fault) = observed.collected {
            faulting += 1;
            if declaration == Some(Declared::Never) {
                uncovered.push(Cow::Owned(format!("{}: {fault:?}", mutation.label)));
            }
        }
    }
    Ok(Coverage {
        faulting,
        uncovered: Cow::Owned(uncovered),
    })
}

/// A kind whose mutations raise `faulting` faults, each one the activation plan reads as able to fault.
const fn covered(faulting: usize) -> Coverage {
    Coverage {
        faulting,
        uncovered: Cow::Borrowed(&[]),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("select_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Select,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("project_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Project,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("extend_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Extend,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("join_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Join,
        expect: Expect::Value(covered(1)),
    },
    Case {
        name: CaseName("union_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Union,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("intersection_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Intersection,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("difference_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Difference,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("antijoin_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Antijoin,
        expect: Expect::Value(covered(2)),
    },
    Case {
        name: CaseName("group_count_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::GroupCount,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("group_digest_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::GroupDigest,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("capture_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Capture,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("rank_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Rank,
        expect: Expect::Value(covered(6)),
    },
    Case {
        name: CaseName("replace_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Replace,
        expect: Expect::Value(covered(2)),
    },
    Case {
        name: CaseName("ordered_match_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::OrderedMatch,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("words_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Words,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("axis_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Axis,
        expect: Expect::Value(covered(10)),
    },
    Case {
        name: CaseName("closure_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Closure,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("ordered_dfs_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::OrderedDfs,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("worklist_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Worklist,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("cycles_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Cycles,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("rows_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Rows,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("hosted_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::Hosted,
        expect: Expect::Value(covered(1)),
    },
    Case {
        name: CaseName("span_between_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::SpanBetween,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("source_matches_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::SourceMatches,
        expect: Expect::Value(covered(5)),
    },
    Case {
        name: CaseName("render_syntax_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::RenderSyntax,
        expect: Expect::Value(covered(0)),
    },
    Case {
        name: CaseName("fits_rendered_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::FitsRendered,
        expect: Expect::Value(covered(2)),
    },
    Case {
        name: CaseName("span_bounds_faults_are_declared"),
        word: Word::Succeeds,
        input: || OperationKind::SpanBounds,
        expect: Expect::Value(covered(0)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
