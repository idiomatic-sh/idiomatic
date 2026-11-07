//! A closed semantic service: program and schema are fixed by request preparation.
//! Matcher acquisition, short circuit and cache lifetime stay inside this owner.
use crate::domain::matching::types::prepared::value::PreparedMatchers;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::PreparedPredicates;
use crate::domain::operator::contract::views::PredicateProgram;
use crate::domain::table_model::types::Verdict;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use {crate::domain::table_texts::types::Texts, ryo_values::domain::files::types::SourceView};

pub(crate) struct BoundPredicate<'a> {
    pub program: &'a PreparedPredicates,
    pub bindings: &'a [ParameterBinding],
    pub source: SourceView<'a>,
    pub paths: &'a [FilePath],
    pub arenas: &'a [ArenaBytes],
    pub prepared: Option<&'a PreparedMatchers>,
    pub state: &'a mut crate::domain::table_scratch::types::LazyScratch<
        crate::domain::table_scratch::types::SelectScratch,
    >,
    pub memo: &'a mut Vec<crate::domain::operating::types::Memo>,
    pub names: &'a mut Option<crate::domain::operating::types::NameCount>,
}
impl PredicateProgram for BoundPredicate<'_> {
    fn holds(&mut self, row: &[Value], symbols: &Symbols) -> Result<Verdict, OperatingError> {
        #[cfg(feature = "diagnostic-row-progress")]
        super::route_counts::progress::service::progress(
            crate::domain::operator::contract::views::ProgressCounter::PredicateEvaluations,
            crate::domain::operator::contract::views::ProgressAmount(1),
        );
        if !self.program.comparisons.is_empty() && row.len() != self.program.kinds.len() {
            return Err(OperatingError::ShapeMismatch);
        }
        let texts = Texts {
            symbols,
            source: self.source,
            paths: self.paths,
            arenas: self.arenas,
        };
        if self.program.comparisons.is_empty() {
            return Ok(Verdict::Holds);
        }
        let names = *self.names.get_or_insert_with(|| {
            let count = crate::domain::operating::types::NameCount(
                ryo_language::domain::names::service::name_count(symbols).0,
            );
            self.memo.clear();
            self.memo.resize(
                self.program.comparisons.len() * count.0,
                crate::domain::operating::types::Memo::Unknown,
            );
            count
        });
        crate::domain::comparing::service::holds_bound(
            &self.program.comparisons,
            row,
            crate::domain::comparing::types::RowShape {
                kinds: &self.program.kinds,
                names,
            },
            &texts,
            self.bindings,
            self.prepared,
            crate::domain::matching::types::PatternMatcher(
                &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
            ),
            crate::domain::comparing::types::DecisionWorkspace {
                select: &mut *self.state,
                memo: &mut *self.memo,
            },
        )
    }
}

#[cfg(test)]
mod constants;

#[cfg(test)]
mod tests {
    use super::*;
    use ryo_relations::domain::relations::types::Relation;
    use ryo_values::domain::plan_parts::types::{
        PlanComparison, Input, LiteralText, Operand, QueryLiteral, Test,
    };
    use ryo_values::domain::plans::types::Operation;
    use std::sync::atomic::Ordering;

    use super::constants::{EMPTY_SOURCE, INVALID_PATTERN, PASS, STOP, SUBJECT};

    #[test]
    fn failed_preceding_predicate_never_acquires_invalid_regex() {
        let comparisons = vec![
            PlanComparison {
                left: Operand::Column(ryo_values::domain::plan_parts::types::ColumnIndex(0)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText(PASS.into()))),
                test: Test::Equal,
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            },
            PlanComparison {
                left: Operand::Literal(QueryLiteral::Text(LiteralText(SUBJECT.into()))),
                right: Operand::Literal(QueryLiteral::Text(LiteralText(INVALID_PATTERN.into()))),
                test: Test::Matches,
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            },
        ];
        let operation = Operation::Select {
            from: Input::Relation(Relation::Comment),
            comparisons: comparisons.clone(),
            kept: vec![],
        };
        let plan = ryo_values::domain::plans::types::Plan {
            parameters: vec![],
            observation_inputs: vec![],
            observations: vec![],
            comparison_origins: vec![],
            frontier: vec![],
            steps: vec![ryo_values::domain::plans::types::PlanStep {
                scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
                operation,
            }],
            emit: ryo_values::domain::plans::types::Emit {
                step: ryo_values::domain::plan_parts::types::StepId(0),
                subject: ryo_values::domain::plan_parts::types::ColumnIndex(0),
                key: vec![],
                headings: vec![],
            },
        };
        let prepared = PreparedMatchers::new(&[plan]);
        let program = PreparedPredicates {
            comparisons,
            kinds: vec![ryo_relations::domain::relations::types::CellKind::Symbol],
        };
        let mut symbols = Symbols::default();
        let stop = ryo_language::domain::symbols::service::intern_derived(
            &mut symbols,
            ryo_language::domain::symbols::types::ByteSlice(EMPTY_SOURCE),
            ryo_relations::domain::terms::types::NameText(STOP),
        );
        let pass = ryo_language::domain::symbols::service::intern_derived(
            &mut symbols,
            ryo_language::domain::symbols::types::ByteSlice(EMPTY_SOURCE),
            ryo_relations::domain::terms::types::NameText(PASS),
        );
        let mut state = Default::default();
        let mut memo = Vec::new();
        let mut names = None;
        let mut evaluator = BoundPredicate {
            program: &program,
            bindings: &[],
            source: SourceView(EMPTY_SOURCE),
            paths: &[],
            arenas: &[],
            prepared: Some(&prepared),
            state: &mut state,
            memo: &mut memo,
            names: &mut names,
        };
        assert_eq!(
            evaluator.holds(&[Value::Symbol(stop)], &symbols).unwrap(),
            Verdict::Fails
        );
        assert_eq!(prepared.builds.load(Ordering::Relaxed), 0);
        assert!(matches!(
            evaluator.holds(&[Value::Symbol(pass)], &symbols),
            Err(OperatingError::InvalidRegex { .. })
        ));
        assert_eq!(prepared.builds.load(Ordering::Relaxed), 1);
    }
}

use ryo_values::domain::query_bindings::types::ParameterBinding;
