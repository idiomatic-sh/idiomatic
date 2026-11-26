//! Translate a typed request's named outputs before its rules are consumed.
use super::{call_cache::CallCache, entry::execution, sdk};
use ryo_executor::{
    check_session::query::Query,
    lower::{Observation, Producer},
};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::observations::types::{ObservationColumn, ObservationColumnName};
use std::{collections::BTreeMap, rc::Rc, sync::Arc};

pub(super) type Inputs = BTreeMap<sdk::RuleId, Rc<[Observation]>>;

pub(super) fn inputs(rules: &BTreeMap<sdk::RuleId, sdk::Rule>) -> Result<Inputs, sdk::LintError> {
    let mut inputs = Inputs::new();
    for (consumer, rule) in rules.iter().filter(|(_, r)| !r.observations.is_empty()) {
        let mut declared = Vec::new();
        for input in &rule.observations {
            let mut producers = Vec::new();
            for producer in &input.producers {
                let sdk::ObservationProducer::Output { rule: id, output } = producer else {
                    return Err(execution(format!(
                        "{consumer}: unavailable observation producer"
                    )));
                };
                let source = rules
                    .get(id)
                    .ok_or_else(|| execution(format!("{consumer}: missing producer {id}")))?;
                if source.include != rule.include
                    || source.exclude != rule.exclude
                    || !source.parameters.is_empty()
                    || !source.observations.is_empty()
                    || (source.applicability != rule.applicability
                        && source.applicability != sdk::RuleApplicability::Always)
                    || super::modules::query_modules(&source.query)
                        != super::modules::query_modules(&rule.query)
                {
                    return Err(execution(format!(
                        "{consumer}: {id} requires a separately scoped observation"
                    )));
                }
                producers.push(Producer {
                    rule: id.as_ref().into(),
                    output: output.0.clone(),
                    text: Arc::from(source.query.as_ref()),
                });
            }
            if producers.is_empty() {
                return Err(execution(format!(
                    "{consumer}: no producer for {}",
                    input.name.0
                )));
            }
            let columns = input
                .columns
                .iter()
                .map(|c| ObservationColumn {
                    name: ObservationColumnName(c.name.0.clone()),
                    kind: match c.kind {
                        sdk::ObservationKind::File => CellKind::File,
                        sdk::ObservationKind::Node => CellKind::Node,
                        sdk::ObservationKind::Symbol => CellKind::Symbol,
                        sdk::ObservationKind::Span => CellKind::Span,
                        sdk::ObservationKind::Flag => CellKind::Flag,
                        sdk::ObservationKind::Number => CellKind::Number,
                    },
                })
                .collect();
            declared.push(Observation {
                name: input.name.0.clone(),
                columns,
                producers,
            });
        }
        inputs.insert(consumer.clone(), Rc::from(declared));
    }
    Ok(inputs)
}

pub(super) fn query(
    cache: &mut CallCache,
    rule: &sdk::Rule,
    inputs: Option<&Rc<[Observation]>>,
) -> Query {
    let (text, digest) = cache.query(rule.query.as_ref());
    Query {
        text,
        digest,
        modules: cache.modules(&rule.query),
        accepted_only: rule.applicability == sdk::RuleApplicability::AcceptedSyntax,
        observations: inputs.cloned().unwrap_or_default(),
    }
}
