//! A closed semantic service: program and schema are fixed by request preparation.
//! Matcher acquisition, short circuit and cache lifetime stay inside this owner.
use crate::domain::matching::types::prepared::value::PreparedMatchers;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{PredicateProgram, PreparedPredicates};
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
        super::route_counts::progress(
            crate::domain::operator::contract::views::ProgressCounter::PredicateEvaluations,
            1,
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
                ryo_language::domain::symbols::service::name_count(symbols).0,
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
            (&self.program.kinds, names),
            &texts,
            self.bindings,
            self.prepared,
            (&mut *self.state, self.memo),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ryo_relations::domain::relations::types::Relation;
    use ryo_values::domain::plan_parts::types::{
        Comparison, Input, LiteralText, Operand, QueryLiteral, Test,
    };
    use ryo_values::domain::plans::types::Operation;
    use std::sync::atomic::Ordering;

    #[test]
    fn failed_preceding_predicate_never_acquires_invalid_regex() {
        let comparisons = vec![
            Comparison {
                left: Operand::Column(ryo_values::domain::plan_parts::types::ColumnIndex(0)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("pass".into()))),
                test: Test::Equal,
                origin: None,
            },
            Comparison {
                left: Operand::Literal(QueryLiteral::Text(LiteralText("subject".into()))),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("[".into()))),
                test: Test::Matches,
                origin: None,
            },
        ];
        let operation = Operation::Select {
            from: Input::Relation(Relation::Comment),
            comparisons: comparisons.clone(),
            kept: vec![],
        };
        let mut plan = crate::domain::stored_selection::fixtures::plan();
        plan.steps.truncate(1);
        plan.steps[0].operation = operation;
        let prepared = PreparedMatchers::new(&[plan]);
        let program = PreparedPredicates {
            comparisons,
            kinds: vec![ryo_relations::domain::relations::types::CellKind::Symbol],
        };
        let mut symbols = Symbols::default();
        let stop = ryo_language::domain::symbols::service::intern_derived(
            &mut symbols,
            ryo_language::domain::symbols::types::ByteSlice(b""),
            ryo_relations::domain::terms::types::NameText("stop"),
        );
        let pass = ryo_language::domain::symbols::service::intern_derived(
            &mut symbols,
            ryo_language::domain::symbols::types::ByteSlice(b""),
            ryo_relations::domain::terms::types::NameText("pass"),
        );
        let mut state = Default::default();
        let mut memo = Vec::new();
        let mut names = None;
        let mut evaluator = BoundPredicate {
            program: &program,
            bindings: &[],
            source: SourceView(b""),
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
