//! What a rule's steps in the open file read besides the file's facts: the tables its bound observations are bound to, from
//! the named outputs of the rules before it in this file, and the host's rows its steps read, interned once per file.

use crate::application::file_dag::file_pass::constants::BOUND_PRODUCER_UNAVAILABLE;
use crate::application::file_dag::file_pass::types::PassFile;
use crate::application::file_dag::frontiers::service::withheld;
use crate::application::file_dag::publications::service::bound::service::admitted;
use crate::application::file_dag::publications::service::lineage::service::output_lineage;
use crate::application::file_dag::publications::service::united;
use crate::application::file_dag::publications::types::{
    BoundRows, BoundTable, BoundTables, MemberRows,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::hosted::service::table_of_query;
use ryo_execution_domain::domain::hosted::types::NumberedFiles;
use ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationProducer;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::AfterTheFiles;
use ryo_language::domain::symbols::types::{QuerySymbols, Symbols};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::{Operation, Plan};
use std::sync::Arc;

/// The tables `rule`'s bound observations in the open file are bound to: each binding's producers' rows in this file, in
/// producer order, each row once; a producer with no rows in this file withholds its binding.
pub(crate) fn in_file_bound(
    at: &mut PassFile<'_, '_, '_, '_>,
    rule: RuleIndex,
) -> Result<BoundTables, OperatingError> {
    let Some(metadata) = at.request.program.observation_metadata() else {
        return Ok(BoundTables::default());
    };
    let Some(bindings) = metadata.bindings.get(rule.0) else {
        return Ok(BoundTables::default());
    };
    let mut tables = Vec::new();
    for binding in bindings
        .iter()
        .filter(|binding| binding.schema.scope == StepScope::PerFile)
    {
        let members = admitted(metadata, rule, binding.input).and_then(|()| {
            binding
                .producers
                .iter()
                .map(|producer| match producer {
                    ResolvedObservationProducer::Output { rule, output } => at
                        .answers
                        .get(rule.0)
                        .and_then(|answers| {
                            answers.get(usize::try_from(output.0).unwrap_or(usize::MAX))
                        })
                        .and_then(Option::as_ref)
                        .ok_or_else(|| OperatingError::ObservationIncomplete {
                            reason: BOUND_PRODUCER_UNAVAILABLE.into(),
                        })
                        .and_then(|answer| answer.as_ref().map_err(Clone::clone))
                        .map(|answer| MemberRows {
                            rows: Arc::new(answer.table.clone()),
                            coverage: answer.coverage.clone(),
                            lineage: output_lineage(at.pass, &metadata.bindings, *rule, *output),
                        }),
                    ResolvedObservationProducer::NoQuery { .. } => {
                        Err(OperatingError::ObservationIncomplete {
                            reason: BOUND_PRODUCER_UNAVAILABLE.into(),
                        })
                    }
                })
                .collect::<Result<Vec<_>, _>>()
        });
        let declared = binding
            .schema
            .columns
            .iter()
            .map(|column| column.kind)
            .collect();
        let rows = match members.and_then(|members| united(&members, declared)) {
            Ok(rows) => rows,
            Err(error) => BoundRows::Withheld(withheld(&mut at.inputs.withholdings, error)),
        };
        tables.push(BoundTable {
            input: binding.input,
            rows,
        });
    }
    Ok(BoundTables(tables))
}

/// The host's rows of each relation a step of `plan` in the open file reads, interned in the file's symbols once per file.
pub(crate) fn hosted_in_file(
    at: &mut PassFile<'_, '_, '_, '_>,
    symbols: &mut Symbols,
    plan: &Plan,
) {
    let declared = at.request.program.hosted();
    for step in plan
        .steps
        .iter()
        .filter(|step| step.scope == StepScope::PerFile)
    {
        let Operation::Hosted { relation } = step.operation else {
            continue;
        };
        let at_place = usize::try_from(relation.0).unwrap_or(usize::MAX);
        let Some(host) = declared.get(at_place) else {
            continue;
        };
        if at.inputs.hosted.len() <= at_place {
            at.inputs
                .hosted
                .resize_with(at_place.saturating_add(1), || None);
        }
        if let Some(held) = at.inputs.hosted.get_mut(at_place)
            && held.is_none()
        {
            *held = Some(table_of_query(
                host,
                NumberedFiles(at.paths),
                &mut QuerySymbols::from(&mut *symbols),
                AfterTheFiles::Before,
            ));
        }
    }
}
