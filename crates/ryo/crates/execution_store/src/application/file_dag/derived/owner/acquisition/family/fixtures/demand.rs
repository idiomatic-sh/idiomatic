//! One acquisition of a family from admitted rows; every scenario issues its demands here.
use super::super::super::admission::service::{AcquiredView, AdmittedViewInputs};
use crate::application::file_dag::relations::service::RelationOwner;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    derived_views::types::{
        DerivedInputs, DerivedScratch, MembershipSpace, contract::value::DerivedContract,
    },
    operating::errors::OperatingError,
    table_cells::fixtures::SMALL_SOURCE,
    table_texts::types::Texts,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::SourceView;
use std::sync::Arc;

pub(super) fn demand(
    kind: DerivedKind,
    owner: &RelationOwner,
    plan: &Arc<DerivedContract>,
    admitted: &AdmittedViewInputs<'_>,
    rows: [RepresentationInput<'_>; 2],
    symbols: &mut Symbols,
) -> Result<AcquiredView, OperatingError> {
    let pair = matches!(
        kind,
        DerivedKind::OrderedKeys | DerivedKind::OrderedDfs | DerivedKind::NativeLookup
    );
    let mut scratch = DerivedScratch::default();
    if kind == DerivedKind::OrderedDfs {
        let mut query = (&mut *symbols).into();
        return owner.derived().acquire_stored(
            plan,
            admitted,
            Default::default(),
            DerivedInputs::Dfs {
                roots: rows[0],
                edges: rows[1],
                symbols: &mut query,
                source: SourceView(SMALL_SOURCE),
                paths: &[],
                arenas: &[],
            },
            &mut scratch,
            None,
        );
    }
    let texts = Texts {
        symbols: &*symbols,
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
    };
    let inputs = match kind {
        DerivedKind::Membership => DerivedInputs::Membership {
            rows: rows[0],
            space: MembershipSpace::Values,
        },
        DerivedKind::OrderedKeys => DerivedInputs::Text {
            rows: &rows,
            texts: &texts,
        },
        DerivedKind::Equality
        | DerivedKind::Adjacency
        | DerivedKind::PartitionOrder
        | DerivedKind::ClosureAdjacency
        | DerivedKind::CycleAdjacency
        | DerivedKind::OrderedDfs
        | DerivedKind::NativeLookup => DerivedInputs::Rows(&rows[..if pair { 2 } else { 1 }]),
    };
    owner.derived().acquire_stored(
        plan,
        admitted,
        Default::default(),
        inputs,
        &mut scratch,
        None,
    )
}
