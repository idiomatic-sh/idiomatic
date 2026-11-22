//! The fixtures of the `assembling` concept: a validated program lowered alone, as the standalone query's.

use super::service::lower_with_observations;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::WrittenDocuments;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::hosted::types as hosted;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::program::types as syntax;
use ryo_values::domain::query_tokens::types::QueryBundle;

/// Lowers a program that validation accepted.
///
/// # Errors
///
/// The first name that resolves to nothing, or comparison of unlike types, with its place.
pub(crate) fn lower(
    program: &syntax::WrittenProgram,
    catalog: &[Relation],
    hosted: &[hosted::HostedRelation],
) -> Result<ryo_values::domain::query_plans::types::Plan, LoweringError> {
    // A program lowered alone is the standalone query's: the identity a bundle of its text alone gives its root.
    let bundle = QueryBundle::from(QueryText(Box::default()));
    let (root, modules) = (&bundle.root.id, &std::collections::BTreeMap::new());
    lower_with_observations(
        program,
        catalog,
        hosted,
        &[],
        WrittenDocuments { root, modules },
    )
}
