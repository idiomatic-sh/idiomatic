//! Why a source could not be read, or a table made, for a file.

use super::types::{MissingCode, UnbuiltFill};
use ryo_language::domain::facts::types::PartialRows;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::Relation;

/// Why the fill side has no rows to give. It names no rule: the loop knows which rule was running and answers the scan's
/// own fault.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum FilledError {
    /// The language refuses the relation for this file.
    #[error("relation {relation:?} refused: {cause:?}")]
    Refused {
        /// The relation.
        relation: Relation,
        /// Why.
        cause: AbsenceCause,
    },
    /// Some rows of the relation may be missing.
    #[error("relation {relation:?} read in part: {rows:?}")]
    Partial {
        /// The relation.
        relation: Relation,
        /// What the filler said.
        rows: PartialRows,
    },
    /// A fill, a stage or a source the fill side does not run yet.
    #[error("the fill side does not run {unbuilt:?} yet")]
    Unbuilt {
        /// What is not built.
        unbuilt: UnbuiltFill,
    },
    /// A per-row meaning of the operators crate faulted on a row: a span out of order or past the file, a group the
    /// expression does not name, a duplicate order in an ordered match.
    #[error("a meaning faulted on a row: {fault:?}")]
    Meaning {
        /// The meaning's fault.
        fault: PortableOperatorFault,
    },
    /// The code names a table, a relation, a site or a pool place it does not hold: the pack is not what the encoding wrote.
    #[error("the code names {named:?}, which it does not hold")]
    Code {
        /// What was named.
        named: MissingCode,
    },
}
