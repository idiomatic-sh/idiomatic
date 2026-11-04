//! Explicit outer scope creation over one required request schedule. Inner
//! operators/observation adapters receive its executor; they cannot mint owners.
use super::{
    executor::{PrimitiveExecutor, ScopedExecutor},
    relations::RelationOwner,
};
use crate::domain::{file_program::types::RequestGraph, operating::errors::OperatingError};
use ryo_relations::domain::relations::types::FileId;
use std::{rc::Rc, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopePhase {
    File(FileId),
    Project,
}
/// A permit's private field prevents unrelated application code from creating a
/// RelationOwner directly. Only this cohesive outer scope constructor mints it.
pub(super) struct ScopePermit {
    _private: (),
}
#[cfg(test)]
impl ScopePermit {
    pub(super) fn fixture() -> Self {
        Self { _private: () }
    }
}

pub(crate) struct ExecutionScope<'a> {
    graph: Arc<RequestGraph>,
    phase: ScopePhase,
    executor: ScopedExecutor<'a>,
}
impl<'a> ExecutionScope<'a> {
    pub(crate) fn new(
        graph: Arc<RequestGraph>,
        phase: ScopePhase,
        stable_population: bool,
        primitive: &'a dyn PrimitiveExecutor,
    ) -> Result<Self, OperatingError> {
        let owner = Rc::new(RelationOwner::for_scope(
            ScopePermit { _private: () },
            stable_population,
            graph
                .nodes
                .len()
                .saturating_add(ryo_relations::domain::relations::service::all().len())
                .saturating_add(1),
            primitive.derived_port(),
        ));
        let executor = ScopedExecutor::new(primitive, owner)?;
        Ok(Self {
            graph,
            phase,
            executor,
        })
    }
    pub(crate) fn executor(&self) -> &ScopedExecutor<'a> {
        &self.executor
    }
    pub(crate) fn owner(&self) -> &Rc<RelationOwner> {
        self.executor.owner()
    }
    pub(crate) fn graph(&self) -> &Arc<RequestGraph> {
        &self.graph
    }
}

pub(crate) enum ScopePartials<'a> {
    File {
        evaluation: &'a [crate::domain::coverage::types::Partial],
        publication: &'a [crate::domain::coverage::types::Partial],
    },
    Project {
        evaluation: &'a crate::domain::coverage::types::Partials,
        publication: &'a crate::domain::coverage::types::Partials,
    },
}
impl ExecutionScope<'_> {
    pub(crate) fn completion<'a>(
        &self,
        rule: crate::domain::coverage::types::RuleIndex,
        partials: ScopePartials<'a>,
    ) -> Result<super::completion::CompletionPolicy, OperatingError> {
        match (self.phase, partials) {
            (
                ScopePhase::File(file),
                ScopePartials::File {
                    evaluation,
                    publication,
                },
            ) => Ok(super::completion::CompletionPolicy::file_inputs(
                rule,
                file,
                evaluation,
                publication,
            )),
            (
                ScopePhase::Project,
                ScopePartials::Project {
                    evaluation,
                    publication,
                },
            ) => Ok(super::completion::CompletionPolicy::project_inputs(
                rule,
                evaluation,
                publication,
            )),
            _ => Err(OperatingError::WrongOperation),
        }
    }
}
