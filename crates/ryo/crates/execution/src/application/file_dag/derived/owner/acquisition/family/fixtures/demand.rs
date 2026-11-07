//! One acquisition of a family from admitted rows; every scenario issues its demands here.
use super::super::super::admission::service::{AcquiredView, AdmittedViewInputs};
use crate::application::file_dag::{predicates::BoundPredicate, relations::RelationOwner};
use crate::domain::{
    derived_views::types::{contract::value::DerivedContract, DerivedInputs, DerivedScratch, MembershipSpace},
    operating::errors::OperatingError,
    operator::contract::{types::PreparedPredicates, },
    positions::service_test::SMALL_SOURCE,
    table_texts::types::Texts,
};
use crate::domain::table_model::types::RepresentationInput;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::{files::types::SourceView, plan_parts::types::LiteralText};
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
    let predicates = PreparedPredicates {
        comparisons: vec![],
        kinds: rows[0].kinds().to_vec(),
    };
    let mut state = Default::default();
    let mut memo = Vec::new();
    let mut names = None;
    let mut predicate = BoundPredicate {
        program: &predicates,
        bindings: &[],
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
        prepared: None,
        state: &mut state,
        memo: &mut memo,
        names: &mut names,
    };
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
    let active = [LiteralText("alpha".into()), LiteralText("beta".into())];
    let texts = Texts {
        symbols: &*symbols,
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
    };
    let inputs = match kind {
        DerivedKind::PredicateMatches => DerivedInputs::Predicate {
            rows: rows[0],
            candidates: None,
            program: &mut predicate,
            symbols: texts.symbols,
        },
        DerivedKind::CandidatePostings => DerivedInputs::Postings {
            rows: rows[0],
            texts: &texts,
            active: &active,
        },
        DerivedKind::DistinctRepresentatives => DerivedInputs::Distinct {
            rows: rows[0],
            positions: crate::domain::stored_selection::types::BorrowedPositions(&[0, 1]),
        },
        DerivedKind::Membership => DerivedInputs::Membership {
            rows: rows[0],
            space: MembershipSpace::Values,
        },
        DerivedKind::OrderedKeys => DerivedInputs::Text {
            rows: &rows,
            texts: &texts,
        },
        DerivedKind::RawKeyOrder => DerivedInputs::RawWords(&crate::domain::positions::types::Keys(vec![2, 1, 2])),
        _ => DerivedInputs::Rows(&rows[..if pair { 2 } else { 1 }]),
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
