//! The texts of the `tree_cells` concept.

/// What a read of a kind's name says when the kind's number names no kind of the table's vocabulary.
pub(super) const KIND_IN_VOCABULARY: &str = "a kind this table's vocabulary names";

/// Authoritative native row readers. Both delivery and completeness admission
/// consume this catalog; there is no independently hard-coded native subset.
pub(super) const NATIVE_RELATIONS: &[(
    ryo_relations::domain::relations::types::Relation,
    super::types::TreeForm,
)] = &[
    (
        ryo_relations::domain::relations::types::Relation::SyntaxTree,
        super::types::TreeForm::Elements,
    ),
    (
        ryo_relations::domain::relations::types::Relation::SyntaxChild,
        super::types::TreeForm::Edges,
    ),
];
