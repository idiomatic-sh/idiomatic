//! Test access to the real selection owner; no mutable backend escapes this module.
use super::*;
use crate::application::file_dag::relations::{RelationOwner, handle::service::PublishedInput};
use crate::domain::{
    coverage::types::Partial,
    operating::types::OperatorContext,
    stored_selection::types::Selector,
    table_model::types::{Rows, SelectedRows},
};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, Relation};
use std::{ops::Deref, rc::Rc, sync::Arc};

pub(crate) struct SelectionFixture {
    owner: Rc<RelationOwner>,
}

#[derive(Clone)]
pub(crate) struct SelectedFixtureRows<'f> {
    pub publication: PublishedInput,
    pub rows: SelectedRows<'f>,
}

impl<'f> Deref for SelectedFixtureRows<'f> {
    type Target = SelectedRows<'f>;
    fn deref(&self) -> &Self::Target {
        &self.rows
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SelectionCounters {
    pub scanned: u64,
    pub candidates: u64,
    pub predicate_cells: u64,
    pub builds: u64,
    pub reuses: u64,
    pub refusals: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SelectionSnapshot {
    pub bytes: usize,
    pub limit: usize,
    pub index_builds: u64,
    pub entries: usize,
    pub selections: usize,
    pub selections_capacity: usize,
    pub owner_metadata_bytes: usize,
    pub optional_bytes: usize,
    pub retained_positions: usize,
    pub retained_positions_capacity: usize,
    pub first_retained_rows_address: Option<usize>,
    pub fanouts: usize,
    pub fanout_payload_bytes: usize,
    pub descriptor_address: Option<usize>,
    pub prepared_descriptor_address: Option<usize>,
    pub prepared_views_address: Option<usize>,
    pub prepared_selectors: usize,
    pub active_repetition_groups: usize,
    pub repetition_capacity: usize,
    pub repeated_discriminator: bool,
    pub counts: SelectionCounters,
}

impl SelectionFixture {
    pub(crate) fn new(
        wanted: &[Relation],
        partials: &[Partial],
        active: Vec<Arc<Selector>>,
        authored: impl IntoIterator<Item = Arc<Selector>>,
        kinds: &'static [CellKind],
    ) -> Result<Self, OperatingError> {
        let plans = wanted
            .iter()
            .copied()
            .map(crate::application::file_dag::derived::owner::fixtures::source_plan)
            .collect::<Vec<_>>();
        Self::prepared(wanted, partials, active, authored, kinds, &plans)
    }
    pub(crate) fn with_graph(
        wanted: &[Relation],
        partials: &[Partial],
        active: Vec<Arc<Selector>>,
        authored: impl IntoIterator<Item = Arc<Selector>>,
        kinds: &'static [CellKind],
    ) -> Result<Self, OperatingError> {
        use ryo_values::domain::{
            plan_parts::types::{ColumnIndex, Input},
            plans::types::Operation,
        };
        let mut plans = wanted
            .iter()
            .copied()
            .map(crate::application::file_dag::derived::owner::fixtures::source_plan)
            .collect::<Vec<_>>();
        if let Some(&relation) = wanted.first() {
            let mut graph =
                crate::application::file_dag::derived::owner::fixtures::source_plan(relation);
            graph.steps[0].operation = Operation::Axis {
                from: Input::Relation(relation),
                edges: Input::Relation(relation),
                domain: Input::Relation(relation),
                direction: ryo_values::domain::plan_parts::types::Direction::Descendant,
                id: ColumnIndex(0),
                parent: ColumnIndex(0),
                child: ColumnIndex(0),
                domain_id: ColumnIndex(0),
                domain_filter: vec![],
                domain_view: vec![],
                output: vec![],
            };
            plans.push(graph);
        }
        Self::prepared(wanted, partials, active, authored, kinds, &plans)
    }
    fn prepared(
        _wanted: &[Relation],
        partials: &[Partial],
        active: Vec<Arc<Selector>>,
        authored: impl IntoIterator<Item = Arc<Selector>>,
        kinds: &'static [CellKind],
        plans: &[ryo_values::domain::plans::types::Plan],
    ) -> Result<Self, OperatingError> {
        let owner = Rc::new(RelationOwner::new(
            crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
            (std::sync::Arc::new(
                ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
            ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        ));
        let descriptors = crate::application::file_dag::derived::owner::fixtures::prepared_sources(
            plans,
            owner.derived().semantic.clone(),
        );
        owner.derived().prepare_program_file(
            &descriptors,
            &(0..plans.len())
                .map(crate::domain::coverage::types::RuleIndex)
                .collect::<Vec<_>>(),
            partials,
        )?;
        owner
            .derived()
            .fixture_install_views(active, authored, kinds)?;
        Ok(Self { owner })
    }

    pub(crate) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
    }

    pub(crate) fn select<'f>(
        &self,
        program: &Arc<Selector>,
        source: ColumnsView<'f>,
        context: &mut OperatorContext<'_>,
    ) -> Result<Option<SelectedFixtureRows<'f>>, OperatingError> {
        let population = self.owner.source(program.relation, &Rows::Stored(source))?;
        let proof = self.owner.admit(
            population.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::Stored(source),
        )?;
        // The real owner inspects the supplied representation before optional
        // selection reuse. A readable ragged backing follows its physical fallback.
        if proof.physical_shape()
            != crate::domain::table_model::collections::representations::types::PhysicalShape::Complete
        {
            return Ok(None);
        }
        let Some(publication) = self.owner.derived().select(
            program,
            source,
            &self.owner,
            &population,
            &proof,
            context,
        )?
        else {
            return Ok(None);
        };
        let rows = self.owner.selected_rows(publication.as_ref(), source)?;
        let Rows::Selected(rows) = rows else {
            unreachable!("selected producer")
        };
        Ok(Some(SelectedFixtureRows { publication, rows }))
    }

    /// Required bytes the owner keeps for the admitted source of `relation` until the generation
    /// retires: what a demand's exit returns to, since the exit does not retire the source.
    pub(crate) fn source_floor(
        &self,
        relation: Relation,
        source: ColumnsView<'_>,
    ) -> Result<usize, OperatingError> {
        let population = self.owner.source(relation, &Rows::Stored(source))?;
        let proof = self.owner.admit(
            population.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::Stored(source),
        )?;
        drop(proof);
        drop(population);
        Ok(self.owner.derived().publication_ownership_bytes().0)
    }

    pub(crate) fn limit_to_metadata(&self) {
        self.owner.derived().fixture_limit_to_metadata()
    }

    pub(crate) fn ineligible(&self) {
        self.owner.derived().fixture_ineligible()
    }

    /// Corrupt request-prepared group metadata before any file selection runs.
    pub(crate) fn corrupt_prepared_group_count(&self) {
        self.owner.derived().fixture_corrupt_prepared_group_count()
    }

    pub(crate) fn reserve_spare_selections(&self, extra: usize) {
        self.owner.derived().fixture_reserve_spare_selections(extra)
    }

    pub(crate) fn snapshot(&self, descriptor: Option<&Arc<Selector>>) -> SelectionSnapshot {
        self.owner.derived().fixture_snapshot(descriptor)
    }
}
