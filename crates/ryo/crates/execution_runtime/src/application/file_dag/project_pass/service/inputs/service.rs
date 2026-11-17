//! What the steps after the files read and borrow: the phase's rows, held for the operators in place, and the phase's
//! resources.

use crate::application::file_dag::frontiers::service::held_frontier;
use crate::application::file_dag::frontiers::types::FrontierRows;
use crate::application::file_dag::project_pass::types::{
    HandoverReads, PhaseInputs, PhaseResources,
};
use crate::application::file_dag::publications::service::bound::service::held_observation;
use ryo_execution_domain::domain::operating::service::minted;
use ryo_execution_domain::domain::operating::types::AfterTheFiles;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::{ByteSlice, QuerySymbols};
use ryo_operators::domain::contract::types::{PortableOperatorFault, WithheldInput};
use ryo_operators::domain::physical_plan::types::activity::value::Activity;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::candidates::value::Candidates;
use ryo_operators::domain::physical_plan::types::fact_rows::value::FactRows;
use ryo_operators::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_operators::domain::physical_plan::types::{
    OpenFileBytes, PlanInputs, PlanNodeId, RenderOutcomes, RenderRequests, Resources,
};
use ryo_operators::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::types::{Relation, Value};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{HostedNumber, Input};
use ryo_values::domain::row_texts::types::CellText;

impl PlanInputs for PhaseInputs<'_> {
    fn facts(&self, _relation: Relation) -> FactRows<'_> {
        // After the files every relation a step reads is handed over to its rule: one no handover answers is not read.
        self.unhanded.set(HandoverReads::Unhanded);
        FactRows::Refused
    }
    fn roots(&self, _root: PlanNodeId) -> Candidates<'_> {
        // No file is open after the files: a step that starts from the tree has no tree node to start from.
        Candidates(&[])
    }
    fn frontier(&self, place: FrontierPlace) -> HeldRows<'_> {
        held_frontier(self.handed, place)
    }
    fn observation(&self, place: BoundObservationPlace) -> HeldRows<'_> {
        // A binding whose producer the stage did not publish is withheld as the frontier entry it is.
        let withheld = self
            .frontier
            .iter()
            .position(|entry| *entry == Input::BoundObservation(place.input))
            .and_then(|at| match self.handed.0.get(at) {
                Some(FrontierRows::Withheld(token)) => Some(*token),
                Some(FrontierRows::Held { .. } | FrontierRows::Elsewhere) | None => None,
            });
        match withheld {
            Some(token) => HeldRows::Withheld(token),
            None => held_observation(self.bound, place),
        }
    }
    fn hosted(&self, relation: HostedNumber) -> HeldRows<'_> {
        match usize::try_from(relation.0).map(|at| self.hosted.0.get(at)) {
            Ok(Some(table)) => HeldRows::Held(table),
            Ok(None) | Err(_) => HeldRows::Withheld(WithheldInput(u32::MAX)),
        }
    }
    fn activity(&self, _node: PlanNodeId) -> Activity<'_> {
        // After the files no file's triggers say a node cannot have rows: every step after the files runs.
        Activity::Active
    }
}

impl Resources for PhaseResources<'_> {
    fn texts(&self) -> Texts<'_> {
        Texts {
            symbols: self.symbols,
            source: SourceView(&[]),
            paths: self.paths,
            arenas: self.arenas,
        }
    }
    fn intern(&mut self, text: CellText<'_>) -> Value {
        self.mint(text)
    }
    fn mint(&mut self, text: CellText<'_>) -> Value {
        // After the files a made text is a name made across the files.
        let symbol = query_intern_derived(
            &mut QuerySymbols::from(&mut *self.symbols),
            ByteSlice(&[]),
            NameText(text.0),
        );
        minted(self.symbols, symbol, AfterTheFiles::After)
    }
    fn source(&self) -> OpenFileBytes<'_> {
        OpenFileBytes(&[])
    }
    fn tree(&self) -> &Elements {
        self.elements
    }
    fn render(
        &mut self,
        _requests: RenderRequests<'_>,
    ) -> Result<RenderOutcomes, PortableOperatorFault> {
        // Rendering is the second stage's: a rule that renders is refused before any file.
        Err(PortableOperatorFault::UnsupportedRender)
    }
}
