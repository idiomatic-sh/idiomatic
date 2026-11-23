//! Compile the catalog's scope and absence dependencies from the already parsed root.
//! File and project evaluation use only the session executor. Its partial inputs retain
//! their typed origins and can support positive rows even where absence is undecidable.
use super::{Program, Result};
use ryo_execution_domain::domain::{
    coverage::{
        service::unknown_message,
        types::{CoverageScope, Partial},
    },
    trust::{
        service::{matters, scoped_needs},
        types::{Bearing, Needs},
    },
};
use ryo_language::domain::files::types::FilePath;
use ryo_query_language::{
    inbound::use_cases::compile::use_case::CompileUseCaseImpl,
    outbound::{catalog, syntax},
};
use ryo_values::domain::{
    hosted::types::HostedRelation,
    plan_parts::types::{QueryText, StepScope},
    query_requests::types::QueryRequest,
};
use std::sync::Arc;

pub(crate) struct Contract {
    pub cross: bool,
    pub frontier: bool,
    pub needs: Needs,
}

pub(crate) fn describe(
    text: &str,
    program: &Program,
    hosted: Vec<HostedRelation>,
) -> Result<Contract> {
    let compiler = CompileUseCaseImpl::new(
        Arc::new(syntax::factory::winnow()),
        Arc::new(catalog::factory::language()),
    );
    let request = QueryRequest {
        text: QueryText(text.into()).into(),
        modules: Default::default(),
        observations: Vec::new(),
        hosted,
    };
    let plan = compiler
        .compile_root(request, (*program.syntax).clone(), Default::default())
        .map_err(|e| e.to_string())?
        .plan;
    Ok(Contract {
        cross: plan.steps[plan.emit.step.0 as usize].scope == StepScope::CrossFile,
        frontier: !plan.frontier.is_empty(),
        needs: scoped_needs(&plan),
    })
}

impl Contract {
    pub(crate) fn report(&self, partials: &[Partial], path: &str) -> Vec<(String, String)> {
        let source = FilePath(path.into());
        partials
            .iter()
            .filter(|p| {
                self.needs
                    .reads
                    .iter()
                    .any(|r| matters(r, p) == Bearing::Matters)
            })
            .map(|p| {
                (
                    path.to_owned(),
                    unknown_message(p, CoverageScope::File(&source))
                        .0
                        .into_string(),
                )
            })
            .collect()
    }
}
