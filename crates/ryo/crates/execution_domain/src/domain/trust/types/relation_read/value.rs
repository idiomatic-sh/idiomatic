//! The `RelationRead` value: the one unit of what a plan demands of the facts, a relation and how far its steps narrow it.
//! Sealed: only `trust` narrows a read or reads its narrowing, so what a read demands can grow here alone.
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::PlanComparison;

/// A place where a plan reads a relation: the relation, and the comparisons on its `kind` column that the rows must satisfy
/// to get any further, when the step that reads it is a `select`.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::trust::types::relation_read::value::RelationRead;
/// use ryo_relations::domain::relations::types::Relation;
/// assert_eq!(RelationRead::whole(Relation::Function).relation(), Relation::Function);
/// ```
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct RelationRead {
    /// The relation read.
    pub(in crate::domain::trust) relation: Relation,
    /// The comparisons on the kind of the rows that this read applies before anything else does.
    pub(in crate::domain::trust) kinds: Vec<PlanComparison>,
}

impl RelationRead {
    /// A read of every row of `relation`, narrowed by nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::trust::types::relation_read::value::RelationRead;
    /// use ryo_relations::domain::relations::types::Relation;
    /// assert_eq!(RelationRead::whole(Relation::Defines), RelationRead::whole(Relation::Defines));
    /// ```
    #[must_use]
    pub fn whole(relation: Relation) -> Self {
        Self {
            relation,
            kinds: Vec::new(),
        }
    }

    /// The relation read.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::trust::types::relation_read::value::RelationRead;
    /// use ryo_relations::domain::relations::types::Relation;
    /// assert_eq!(RelationRead::whole(Relation::Defines).relation(), Relation::Defines);
    /// ```
    #[must_use]
    pub fn relation(&self) -> Relation {
        self.relation
    }
}
