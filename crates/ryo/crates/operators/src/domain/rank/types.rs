//! What a `rank` lowers to.

/// What a rank lowers to: the text column and its layers, resolved from the bindings at planning.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RankParts {
    /// The text column.
    pub(crate) column: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The layers, or the fault resolving them raises at the first row.
    pub(crate) layers: RankLayers,
}

/// The layers of a rank, resolved once from what its parameter is bound to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RankLayers {
    /// The sets of texts, in order.
    Bound(Vec<Vec<ryo_values::domain::plan_parts::types::LiteralText>>),
    /// The parameter has no binding, or a binding of another shape: raised at the first row.
    Refused(crate::domain::contract::types::PortableOperatorFault),
}
