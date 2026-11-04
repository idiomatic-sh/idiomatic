//! Project source checks and invocation context; execution belongs to the common program.
use super::{cross_execution::evaluate_cross_roots, program::facade, settings::CrossingInput};
use crate::application::cross_file::types::{FrontierInput, PreparedFrontiers};
use crate::domain::{
    answering::types::Todo,
    coverage::types::Partials,
    crossing::{
        service::{partials_for, partials_for_reads},
        types::Evaluated,
    },
    finish::types::{Arenas, Resolving},
    rules::{errors::RulesError, service::failed},
    spelling::service::check_name_spellings,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Name, Value};

pub(crate) fn cross_rule_with_inputs(
    crossing: &CrossingInput<'_>,
    todo: &Todo,
    mut frontiers: PreparedFrontiers,
    bound_inputs: &[crate::domain::crossing::types::BoundInputWords],
    cross: &mut Symbols,
    publication: &facade::PublicationScope,
) -> Result<Evaluated, RulesError> {
    let refuse = |error: &dyn std::fmt::Display| failed(&todo.id, error);
    check_name_spellings(
        frontiers
            .rows
            .iter()
            .flat_map(|input| -> Box<dyn Iterator<Item = Name> + '_> {
                match input {
                    FrontierInput::Decoded {
                        frontier: input, ..
                    } => Box::new(input.table.cells.iter().filter_map(|value| match value {
                        Value::Name(name) => Some(*name),
                        _ => None,
                    })),
                    FrontierInput::Compact { spelling, .. } => Box::new(spelling.iter().copied()),
                }
            }),
        &Resolving {
            paths: crossing.paths,
            arenas: Arenas(crossing.arenas),
            cross,
            unknown: crossing.unknown,
        },
    )
    .map_err(|error| refuse(&error))?;
    for input in &mut frontiers.rows {
        if let FrontierInput::Compact { spelling, .. } = input {
            *spelling = Vec::new();
        }
    }
    let evaluation = match crossing.answering {
        crate::domain::render::types::Answering::Findings => {
            partials_for(&crossing.domain, todo.rule)
        }
        crate::domain::render::types::Answering::Rows(_) => Partials::default(),
    };
    let reads = crossing
        .program
        .observation_reads(todo.rule)
        .map_err(|error| refuse(&error))?;
    let publication_partials = partials_for_reads(&crossing.domain, todo.rule, &reads);
    let required = facade::required_partitions(
        &crossing.program,
        todo.rule,
        crossing.files,
        crossing.covered,
        crossing.skipped,
        crossing.services,
    )
    .map_err(|error| refuse(&error))?;
    evaluate_cross_roots(
        &crossing.program,
        crossing.services,
        todo.rule,
        required,
        facade::ScopePartials::Project {
            evaluation: &evaluation,
            publication: &publication_partials,
        },
        facade::ProjectFrontiers {
            ordinary: frontiers,
            bound_inputs,
            publication,
        },
        cross,
        crossing.paths,
        crossing.arenas,
    )
    .map_err(|error| refuse(&error))
}
