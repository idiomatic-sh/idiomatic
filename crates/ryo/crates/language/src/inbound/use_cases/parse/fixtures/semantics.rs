//! Execute source-backed semantic-session fixtures through the same provider boundary as a file.

use super::super::service::build;
use super::super::types::{FieldRequest, NoteRequest, ParseEngine, ParseRequest};
use super::expected::SemanticExpected;
use super::findings::Findings;
use crate::domain::elements::service::{end_at, positions, start_at};
use crate::domain::elements::types::Elements;
use crate::domain::semantics::types::{RenderOutcome, RenderRequest, SemanticSlot};
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::terms::types::UnknownReason;

/// Observe each complete semantic answer; only actual unsupported outcomes satisfy a reason claim.
pub(super) fn judge(
    parse: &dyn ParseEngine,
    cases: &[SemanticExpected],
    findings: &mut Findings,
) -> Vec<UnknownReason> {
    let mut observed = Vec::new();
    for case in cases {
        let file = FileId(0);
        let request = ParseRequest {
            path: case.source.path,
            file,
            bytes: case.source.bytes,
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        };
        let (mut table, mut symbols) = (Elements::default(), Symbols::default());
        if let Err(error) = build(parse, request, &mut table, &mut symbols) {
            findings.0.push(format!(
                "{}: semantic fixture parse failed: {error}",
                case.source.path
            ));
            continue;
        }
        let matches: Vec<_> = positions(&table)
            .filter(|&at| start_at(&table, at) == case.start && end_at(&table, at) == case.end)
            .collect();
        let [at] = matches.as_slice() else {
            findings.0.push(format!(
                "{}: semantic fixture span {:?}..{:?} identifies {} nodes, expected one",
                case.source.path,
                case.start,
                case.end,
                matches.len()
            ));
            continue;
        };
        let mut begun = SemanticSlot::default();
        parse.begin_semantics(&request, &table, &mut begun);
        let SemanticSlot::Begun(mut session) = begun else {
            findings.0.push(format!(
                "{}: no semantic session for declared fixture",
                case.source.path
            ));
            continue;
        };
        let asked = RenderRequest {
            node: node_id(file, *at),
            context: case.context,
        };
        match session.render_batch(case.source.bytes, &[asked]) {
            Ok(answers) if answers.len() == 1 => {
                let answer = &answers[0];
                if answer != &case.outcome {
                    findings.0.push(format!(
                        "{}: semantic fixture answered {answer:?}, expected {:?}",
                        case.source.path, case.outcome
                    ));
                }
                if let RenderOutcome::Unsupported(reason) = answer {
                    if parse
                        .unknown_reasons()
                        .iter()
                        .any(|name| name.reason == *reason)
                    {
                        observed.push(*reason);
                    } else {
                        findings.0.push(format!(
                            "{}: semantic fixture used undeclared unknown reason number {}",
                            case.source.path, reason.0
                        ));
                    }
                }
            }
            Ok(answers) => findings.0.push(format!(
                "{}: semantic fixture returned {} answers for one request",
                case.source.path,
                answers.len()
            )),
            Err(error) => findings.0.push(format!(
                "{}: semantic fixture rendering failed: {error}",
                case.source.path
            )),
        }
    }
    observed
}

#[cfg(test)]
#[path = "semantics_test.rs"]
mod tests;
