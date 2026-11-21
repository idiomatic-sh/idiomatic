//! The sites of a fixture pack's indexes, as the code numbers them.

use crate::domain::code::service::list;
use crate::domain::code::types::{IndexSite, ProgramCode};

/// The first site of an index over the node table, as the code numbers it; a site past every site when it holds none.
pub(crate) fn node_site(code: &ProgramCode) -> IndexSite {
    let found = code.indexes.iter().position(|index| index.source.0 == 0);
    IndexSite(
        found
            .and_then(|at| u32::try_from(at).ok())
            .unwrap_or(u32::MAX),
    )
}

/// The site of the index of a source (0 the node table, 1 a relation, 2 a made table) by the columns, as the code
/// numbers it; a site past every site when the code holds none.
pub(crate) fn site_of(code: &ProgramCode, source: u16, columns: &[u32]) -> IndexSite {
    let found = code.indexes.iter().position(|index| {
        index.source.0 == source
            && list(&code.pool, index.columns)
                .iter()
                .map(|slot| slot.0)
                .eq(columns.iter().copied())
    });
    IndexSite(
        found
            .and_then(|at| u32::try_from(at).ok())
            .unwrap_or(u32::MAX),
    )
}
