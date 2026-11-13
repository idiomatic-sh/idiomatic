//! `evaluate` over the fused program of a request.

use super::interface::EvaluateUseCase;
use super::types::EvaluateInput;
use crate::application::file_session::service::Session;
use crate::application::file_session::types::Program;
use crate::domain::batch::service::each;
use crate::domain::fused::errors::FusedError;
use crate::domain::fused::service::fuse;
use crate::outbound::facts::interface::Facts;
use ryo_execution::inbound::use_cases::execution::interface::{
    DiagnosticDelivery, EmittedRoot, FileEnd, FileRuleIntent, FileRules, FinishedReadiness,
    RanRule, RootOutcome, RuleAfter, RuleIndex,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
use ryo_matching::inbound::use_cases::matching::types::RegexSource;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::rows::types::Table;

/// `evaluate` over one request's rules.
#[derive(Debug)]
pub struct EvaluateUseCaseImpl {
    program: Program,
    order: Vec<RuleIndex>,
}

impl EvaluateUseCaseImpl {
    /// Fuses `plans`, one for each rule by its place, run in `order`, over a language whose kinds are named `kinds` by number.
    ///
    /// # Errors
    ///
    /// What fusing refuses: a rule with parameters, or a step the spike does not evaluate.
    pub fn new(plans: &[Plan], order: Vec<RuleIndex>, kinds: &[&str]) -> Result<Self, FusedError> {
        let fused = fuse(plans, kinds)?;
        let matching = MatchingUseCaseImpl::new();
        let regexes = fused
            .nodes
            .iter()
            .map(|operation| match operation {
                Operation::Replace { pattern, .. } | Operation::SourceMatches { pattern, .. } => {
                    Some(
                        matching
                            .compile_regex(RegexSource(&pattern.0))
                            .map_err(|error| error.to_string()),
                    )
                }
                _ => None,
            })
            .collect();
        Ok(Self {
            program: Program {
                fused,
                regexes,
                operators: OperatorsUseCaseImpl::new(std::sync::Arc::new(
                    ryo_operators::outbound::matching::factory::use_case(),
                )),
                matching,
            },
            order,
        })
    }
}

/// The kind of what a value holds.
fn kind_of(value: Value) -> CellKind {
    match value {
        Value::Node(_) => CellKind::Node,
        Value::Symbol(_) | Value::Name(_) => CellKind::Symbol,
        Value::File(_) => CellKind::File,
        Value::Span(_) => CellKind::Span,
        Value::Number(_) => CellKind::Number,
        Value::Flag(_) => CellKind::Flag,
    }
}

impl EvaluateUseCase for EvaluateUseCaseImpl {
    fn evaluate(
        &self,
        input: EvaluateInput<'_>,
        symbols: &mut Symbols,
        facts: &dyn Facts,
    ) -> FileRules {
        let mut session = Session::new(&self.program, facts, symbols, input.source);
        let mut rules = Vec::new();
        for rule in &self.order {
            let Some(FileRuleIntent::Run { diagnostic, .. }) = input.intents.get(rule.0) else {
                continue;
            };
            let emit = if *diagnostic == DiagnosticDelivery::Disabled {
                EmittedRoot::Withheld
            } else {
                let outcome = match self.program.fused.emits.get(rule.0) {
                    Some(node) => match session.rows(*node) {
                        Ok(rows) => RootOutcome::Complete(Table {
                            kinds: each(rows)
                                .next()
                                .map(|row| row.iter().copied().map(kind_of).collect())
                                .unwrap_or_default(),
                            cells: rows.cells.clone(),
                            count: RowCount(rows.count),
                        }),
                        Err(error) => RootOutcome::Failed(error),
                    },
                    None => RootOutcome::NoQuery,
                };
                EmittedRoot::Delivered(outcome)
            };
            rules.push(RanRule {
                rule: *rule,
                emit,
                after: RuleAfter::Ran {
                    captures: Vec::new(),
                },
            });
        }
        FileRules {
            rules,
            end: FileEnd::Finished(FinishedReadiness::Unobserved),
        }
    }
}
