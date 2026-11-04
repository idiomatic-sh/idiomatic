//! Consume admitted named publications in this rule's symbol lifetime; never evaluate a producer here.
use crate::application::file_dag::{observations::bound, settings::CrossingInput};
use crate::domain::{
    coverage::types::RuleIndex,
    crossing::types::BoundInputWords,
    operating::{errors::OperatingError, types::OperatorContext},
    table_model::types::Frontier,
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::Input;

pub(super) fn observed(
    crossing: &CrossingInput<'_>,
    rule: RuleIndex,
    input: &BoundInputWords,
    symbols: &mut Symbols,
) -> Result<Frontier, OperatingError> {
    let words = input.words.as_ref().map_err(Clone::clone)?;
    let tables = bound::decoded(words, symbols)?;
    let bindings = crossing
        .bindings
        .get(rule.0)
        .ok_or(OperatingError::NotRun)?;
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings,
        symbols: symbols.into(),
        source: SourceView(&[]),
        paths: crossing.paths,
        arenas: crossing.arenas,
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let table = bound::combine(
        tables,
        &input.kinds,
        input.union.as_ref().map_err(Clone::clone)?,
        crossing.executor,
        &mut context,
    )?;
    Ok(Frontier {
        input: Input::BoundObservation(input.input),
        table,
    })
}
