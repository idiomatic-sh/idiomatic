//! The `trust` service's read sets: what each step of a plan reads, each set stored once per program.

use super::super::service::{kind_place, needs_of, physical_reads, step_read_sets};
use super::super::types::read_sets::value::{ReadSets, StepReadSets};
use super::super::types::relation_read::value::RelationRead;
use super::super::types::{SetSharing, StoredSets};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as PlanInput, LiteralText, Operand, PlanComparison,
    QueryLiteral, StepId, StepScope, Test,
};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};

/// What a row asks of the read sets.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// Whether every step of the plan holds the stored uses its first step holds, and how many sets the program stores.
    Sharing(fn() -> Plan),
    /// How many steps of the plan have read sets equal to what `physical_reads` and the transitive `needs_of` answer.
    Agreement(fn() -> Plan),
    /// How many uses and needs one step of the plan holds.
    StepReads(fn() -> Plan, usize),
    /// Whether the first step of the second plan holds the stored uses the first step of the first plan holds, both plans' sets
    /// stored in one program, and how many sets the program stores.
    Program(fn() -> Plan, fn() -> Plan),
}

/// What the read sets answer.
#[derive(Debug, PartialEq)]
enum Answer {
    /// Whether every step's uses is its first step's stored set, and how many sets the program stores.
    Stored {
        first: SetSharing,
        distinct: StoredSets,
    },
    /// Of the steps compared, how many disagree.
    Agreeing { steps: usize, disagreeing: usize },
    /// How many reads a step's uses and needs hold.
    Reads { uses: usize, needs: usize },
}

type Input = Ask;
type Value = Answer;
type Failure = std::convert::Infallible;

fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::Sharing(plan) => {
            let mut sets = ReadSets::default();
            let steps = step_read_sets(&plan(), &mut sets).0;
            let first = match steps.split_first() {
                Some((first, rest))
                    if rest
                        .iter()
                        .all(|each| each.uses.sharing(&first.uses) == SetSharing::Shared) =>
                {
                    SetSharing::Shared
                }
                Some(_) | None => SetSharing::Copied,
            };
            Answer::Stored {
                first,
                distinct: sets.distinct(),
            }
        }
        Ask::Agreement(plan) => {
            let plan = plan();
            let made = step_read_sets(&plan, &mut ReadSets::default()).0;
            let expected = expected_reads(&plan);
            Answer::Agreeing {
                steps: made.len().max(expected.len()),
                disagreeing: made
                    .iter()
                    .zip(&expected)
                    .filter(|(made, (uses, needs))| *made.uses != **uses || *made.needs != **needs)
                    .count()
                    + made.len().abs_diff(expected.len()),
            }
        }
        Ask::Program(one, two) => {
            let mut sets = ReadSets::default();
            let one = step_read_sets(&one(), &mut sets).0;
            let two = step_read_sets(&two(), &mut sets).0;
            Answer::Stored {
                first: match (one.first(), two.first()) {
                    (Some(one), Some(two)) => two.uses.sharing(&one.uses),
                    (Some(_) | None, None) | (None, Some(_)) => SetSharing::Copied,
                },
                distinct: sets.distinct(),
            }
        }
        Ask::StepReads(plan, at) => {
            let steps = step_read_sets(&plan(), &mut ReadSets::default()).0;
            let each = steps.get(at).cloned().unwrap_or_else(StepReadSets::default);
            Answer::Reads {
                uses: each.uses.len(),
                needs: each.needs.len(),
            }
        }
    })
}

/// What each step of `plan` reads as the per-step reads were worked out before they were stored once: its physical reads (none
/// for a refused step), and the reads it trusts the absence of together with those of every step it reads.
fn expected_reads(plan: &Plan) -> Vec<(Vec<RelationRead>, Vec<RelationRead>)> {
    let uses = physical_reads(plan);
    let trusted = needs_of(plan);
    let mut reads: Vec<(Vec<RelationRead>, Vec<RelationRead>)> = Vec::new();
    for (at, step) in plan.steps.iter().enumerate() {
        // A refused step reads nothing.
        let own = if let Some(Ok(read)) = uses.get(at) {
            read.clone()
        } else {
            Vec::new()
        };
        let mut needs: Vec<RelationRead> = Vec::new();
        let mut more: Vec<RelationRead> = trusted.get(at).cloned().unwrap_or_default();
        for input in named(&inputs_of(&step.operation)) {
            if let PlanInput::Step(id) = input
                && let Some((_, earlier)) = usize::try_from(id.0).map_or(None, |at| reads.get(at))
            {
                more.extend(earlier.iter().cloned());
            }
        }
        for read in more {
            if !needs.contains(&read) {
                needs.push(read);
            }
        }
        reads.push((own, needs));
    }
    reads
}

/// A per-file step of `operation`.
fn step(operation: Operation) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation,
    }
}

/// A plan of `steps` that emits its last step.
fn emitting(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        emit: Emit {
            step: StepId(u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX)),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        steps,
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        frontier: Vec::new(),
    }
}

/// Every column of `from`, compared by nothing.
fn passed(from: PlanInput) -> Operation {
    Operation::Select {
        from,
        comparisons: Vec::new(),
        kept: Vec::new(),
    }
}

/// The rows of `from` whose `kind` is `kind`, every column kept.
fn of_kind(from: PlanInput, kind: &str) -> Operation {
    Operation::Select {
        from,
        comparisons: vec![PlanComparison {
            test: Test::Equal,
            left: Operand::Column(kind_place(Relation::Defines).unwrap_or(ColumnIndex(0))),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(kind.into()))),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: Vec::new(),
    }
}

/// Step `at` of the plan.
fn from_step(at: u32) -> PlanInput {
    PlanInput::Step(StepId(at))
}

/// Fifty selects over `defines`, each of the one before, none narrowing a kind.
fn chained() -> Plan {
    let mut steps = vec![step(passed(PlanInput::Relation(Relation::Defines)))];
    for at in 0..49 {
        steps.push(step(passed(from_step(at))));
    }
    emitting(steps)
}

/// A select over `defines`, then the rows of it whose kind is `enum`.
fn narrowed() -> Plan {
    emitting(vec![
        step(passed(PlanInput::Relation(Relation::Defines))),
        step(of_kind(from_step(0), "enum")),
    ])
}

/// The functions with no definition of the same name, then their name alone: the antijoin trusts the absence of the
/// definitions, and the projection reads the antijoin.
fn trusting() -> Plan {
    emitting(vec![
        step(passed(PlanInput::Relation(Relation::Function))),
        step(of_kind(PlanInput::Relation(Relation::Defines), "enum")),
        step(Operation::Antijoin {
            left: from_step(0),
            right: from_step(1),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        }),
        step(Operation::Project {
            from: from_step(2),
            columns: vec![ColumnIndex(0)],
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
        }),
    ])
}

/// A select over `defines`, then an antijoin of it with a step the plan does not have before it: an authoring error.
fn refused() -> Plan {
    emitting(vec![
        step(passed(PlanInput::Relation(Relation::Defines))),
        step(Operation::Antijoin {
            left: from_step(0),
            right: from_step(5),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        }),
    ])
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("steps_that_rest_on_equal_reads_hold_one_stored_set"),
        word: Word::Succeeds,
        input: || Ask::Sharing(chained),
        expect: Expect::Value(Answer::Stored {
            first: SetSharing::Shared,
            distinct: StoredSets(2),
        }),
    },
    Case {
        name: CaseName("rules_that_rest_on_equal_reads_hold_one_stored_set"),
        word: Word::Succeeds,
        input: || Ask::Program(chained, narrowed),
        expect: Expect::Value(Answer::Stored {
            first: SetSharing::Shared,
            distinct: StoredSets(3),
        }),
    },
    Case {
        name: CaseName("a_narrowed_read_is_a_set_of_its_own"),
        word: Word::Succeeds,
        input: || Ask::Sharing(narrowed),
        expect: Expect::Value(Answer::Stored {
            first: SetSharing::Copied,
            distinct: StoredSets(3),
        }),
    },
    Case {
        name: CaseName("read_sets_answer_what_physical_reads_and_needs_of_answer_over_a_chain"),
        word: Word::Succeeds,
        input: || Ask::Agreement(chained),
        expect: Expect::Value(Answer::Agreeing {
            steps: 50,
            disagreeing: 0,
        }),
    },
    Case {
        name: CaseName("read_sets_answer_what_physical_reads_and_needs_of_answer_over_a_narrowing"),
        word: Word::Succeeds,
        input: || Ask::Agreement(narrowed),
        expect: Expect::Value(Answer::Agreeing {
            steps: 2,
            disagreeing: 0,
        }),
    },
    Case {
        name: CaseName(
            "read_sets_answer_what_physical_reads_and_needs_of_answer_over_a_trusted_absence",
        ),
        word: Word::Succeeds,
        input: || Ask::Agreement(trusting),
        expect: Expect::Value(Answer::Agreeing {
            steps: 4,
            disagreeing: 0,
        }),
    },
    Case {
        name: CaseName("read_sets_answer_what_physical_reads_and_needs_of_answer_over_a_refusal"),
        word: Word::Succeeds,
        input: || Ask::Agreement(refused),
        expect: Expect::Value(Answer::Agreeing {
            steps: 2,
            disagreeing: 0,
        }),
    },
    Case {
        name: CaseName("a_step_after_a_trusted_absence_needs_what_it_reads_needs"),
        word: Word::Succeeds,
        input: || Ask::StepReads(trusting, 3),
        expect: Expect::Value(Answer::Reads { uses: 2, needs: 1 }),
    },
    Case {
        name: CaseName("a_refused_step_reads_nothing"),
        word: Word::Succeeds,
        input: || Ask::StepReads(refused, 1),
        expect: Expect::Value(Answer::Reads { uses: 0, needs: 0 }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
