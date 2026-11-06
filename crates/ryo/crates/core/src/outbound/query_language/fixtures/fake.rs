//! The fake of the `query_language` port: what the core's tests stand in for it with.

use std::sync::atomic::{AtomicUsize, Ordering};

use ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguageError;
use ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguagePort;
use ryo_relations::domain::relations::types::{Integer, Offset, Relation, Span};
use ryo_values::domain::descriptions::types::{RelationName, StepCount, ValidateReport};
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::program::types::RefusalMessage;
use ryo_values::domain::program::types::RefusedKind;
use ryo_values::domain::program::types::RefusedQuery;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input, LiteralText, Operand, QueryLiteral, StepId, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::plan_parts::types::StepScope;

/// What the core's tests stand in for `query_language` with: it compiles any text to the plan it
/// was given, validates any text as reading the relations the host declared, refuses text that contains `refuse`, and counts how many
/// times it was asked to compile.
#[derive(Debug)]
pub struct QueryLanguageFake {
    /// The plan every acceptable text compiles to.
    pub plan: Plan,
    /// How many times `compile` was called.
    pub calls: AtomicUsize,
}

impl Default for QueryLanguageFake {
    /// A fake whose plan has no steps.
    fn default() -> Self {
        Self {
            plan: Plan {
                observation_inputs: Vec::new(),
                observations: Vec::new(),
                parameters: Vec::new(),
                steps: Vec::new(),
                emit: Emit {
                    step: StepId(0),
                    subject: ColumnIndex(0),
                    key: Vec::new(),
                    headings: Vec::new(),
                },
                frontier: Vec::new(),
                comparison_origins: Vec::new(),
            },
            calls: AtomicUsize::new(0),
        }
    }
}

/// A per-file step over `function` that keeps the rows `comparisons` admit.
fn select(comparisons: Vec<Comparison>) -> PlanStep {
    PlanStep {
        operation: Operation::Select {
            from: Input::Relation(Relation::Function),
            comparisons,
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    }
}

/// A plan of the steps given, emitting the step `step` as the rows of `function` are shown.
fn plan_of(steps: Vec<PlanStep>, step: u32) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps,
        emit: Emit {
            step: StepId(step),
            subject: ColumnIndex(2),
            key: vec![ColumnIndex(3)],
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

impl QueryLanguageFake {
    /// A fake whose plan gives every row of `function`.
    #[must_use]
    pub fn selecting() -> Self {
        Self {
            plan: plan_of(vec![select(Vec::new())], 0),
            ..Self::default()
        }
    }

    /// A fake whose plan reads `function` in two steps and emits the second.
    #[must_use]
    pub fn selecting_twice() -> Self {
        Self {
            plan: plan_of(vec![select(Vec::new()), select(Vec::new())], 1),
            ..Self::default()
        }
    }

    /// A fake whose plan compares the kind of the nodes of the syntax tree with a word that names no kind.
    #[must_use]
    pub fn comparing_a_kind() -> Self {
        let comparison = Comparison {
            test: Test::Equal,
            left: Operand::Column(ColumnIndex(1)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText("no-such-kind".into()))),
            origin: None,
        };
        let mut step = select(vec![comparison]);
        if let Operation::Select { from, .. } = &mut step.operation {
            *from = Input::Relation(Relation::SyntaxTree);
        }
        Self {
            plan: plan_of(vec![step], 0),
            ..Self::default()
        }
    }

    /// A fake whose plan compares a column the rows of `function` do not have.
    #[must_use]
    pub fn comparing_a_missing_column() -> Self {
        let comparison = Comparison {
            test: Test::Equal,
            left: Operand::Column(ColumnIndex(9)),
            right: Operand::Literal(QueryLiteral::Number(Integer(1))),
            origin: None,
        };
        Self {
            plan: plan_of(vec![select(vec![comparison])], 0),
            ..Self::default()
        }
    }

    /// A fake whose plan counts the files each name is in: the count groups by a name, so it is a cross-file step, and the
    /// per-file select hands it every row.
    #[must_use]
    pub fn counting_across_files() -> Self {
        let count = PlanStep {
            operation: Operation::GroupCount {
                from: Input::Step(StepId(0)),
                keys: vec![ColumnIndex(1)],
                distinct: None,
            },
            scope: StepScope::CrossFile,
        };
        let mut plan = plan_of(vec![select(Vec::new()), count], 1);
        plan.emit.subject = ColumnIndex(0);
        plan.emit.key = vec![ColumnIndex(1)];
        plan.frontier = vec![Input::Step(StepId(0))];
        Self {
            plan,
            ..Self::default()
        }
    }
}

impl QueryLanguagePort for QueryLanguageFake {
    fn validate(
        &self,
        bundle: &ryo_values::domain::program::types::QueryBundle,
        hosted: &[HostedRelation],
        _observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ValidateReport, QueryLanguageError> {
        if bundle.root.text.contains(REFUSED) {
            return Err(refused());
        }
        Ok(ValidateReport {
            relations: hosted
                .iter()
                .map(|each| RelationName(each.name.0.clone()))
                .collect(),
            operators: Vec::new(),
            steps: StepCount(self.plan.steps.len()),
        })
    }

    fn compile(
        &self,
        bundle: &ryo_values::domain::program::types::QueryBundle,
        _hosted: &[HostedRelation],
        _observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError> {
        let _calls_before = self.calls.fetch_add(1, Ordering::SeqCst);
        if bundle.root.text.contains(REFUSED) {
            return Err(refused());
        }
        Ok(self.plan.clone())
    }
}

/// What the fake says of a text that contains `refuse`.
fn refused() -> QueryLanguageError {
    QueryLanguageError::Refused {
        error: RefusedQuery {
            kind: RefusedKind::Invalid,
            message: RefusalMessage("the fake refuses text that contains `refuse`".into()),
            span: Span {
                start: Offset(0),
                end: Offset(0),
            },
            origin: None,
        },
    }
}

/// The text the fake refuses.
pub const REFUSED: &str = "refuse";

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
