//! The runner over the physical plan's kernels and the matching port the edge composed.
use super::interface::RunUseCase;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{PlanInputs, PlanNodeId, Resources};
use crate::outbound::matching::interface::MatchingPort;
use std::sync::Arc;

/// The runner over the port that finds regex occurrences; the edge composes it once.
#[derive(Clone)]
pub struct RunUseCaseImpl {
    /// Finds the occurrences of a prepared regex in a text.
    matching: Arc<dyn MatchingPort>,
}

impl std::fmt::Debug for RunUseCaseImpl {
    /// The use case for a debug print: its port is a trait object with no `Debug`, so it names only itself.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl RunUseCaseImpl {
    /// The runner composed once at an edge, with the port that finds regex occurrences.
    #[must_use]
    pub fn new(matching: Arc<dyn MatchingPort>) -> Self {
        Self { matching }
    }
}

impl RunUseCase for RunUseCaseImpl {
    fn run(
        &self,
        plan: &PhysicalPlan,
        node: PlanNodeId,
        buffers: &mut FileBuffers,
        inputs: &dyn PlanInputs,
        resources: &mut dyn Resources,
    ) -> Result<(), PortableOperatorFault> {
        let semantics =
            crate::application::matching::types::PortSemantics(self.matching.semantics());
        crate::domain::physical_plan::service::run::service::run(
            plan,
            &mut crate::domain::physical_plan::types::KernelRun {
                inputs,
                resources,
                matcher: crate::domain::patterns::types::PatternMatcher(&semantics),
            },
            node,
            buffers,
        )
    }
}

impl crate::domain::contract::types::Occurrences for RunUseCaseImpl {
    fn matches(
        &self,
        regex: &ryo_matching::inbound::use_cases::matching::types::CompiledRegex,
        text: ryo_matching::inbound::use_cases::matching::types::SearchText<'_>,
        each: &mut dyn FnMut(
            ryo_matching::inbound::use_cases::matching::types::TextSpan,
        ) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()> {
        self.matching.visit_regex_matches(regex, text, each)
    }
    fn captures(
        &self,
        regex: &ryo_matching::inbound::use_cases::matching::types::CompiledRegex,
        text: ryo_matching::inbound::use_cases::matching::types::SearchText<'_>,
        group: ryo_matching::inbound::use_cases::matching::types::GroupName<'_>,
        each: &mut dyn FnMut(
            ryo_matching::inbound::use_cases::matching::types::TextSpan,
            ryo_matching::inbound::use_cases::matching::types::TextSpan,
        ) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, PortableOperatorFault> {
        self.matching
            .visit_regex_captures(regex, text, group, each)
            .map_err(|_| PortableOperatorFault::MissingCapture)
    }
}
