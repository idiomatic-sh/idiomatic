//! The views of a partitioned closure: its shared ordering is admitted and acquired, and each next partition admitted and
//! built from it, here in the derived concept, which holds both leases for the cursor (`closure_partitions`).
use crate::application::file_dag::closure_partitions::service::PartitionedClosure;
use crate::application::file_dag::closure_partitions::types::{OrderCursor, PartitionAdvance};
use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use crate::application::file_dag::derived::owner::service::DerivedOwner;
use crate::application::file_dag::derived::partition_views::types::PartitionViews;
use crate::application::file_dag::relations::service::RelationOwner;
use ryo_derived::domain::closure_adjacency::types::ClosurePartition;
use ryo_execution_domain::domain::derived_views::types::contract::value::DerivedContract;
use ryo_execution_domain::domain::derived_views::types::{
    DerivedInputs, DerivedScratch, ViewRequest,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use std::{rc::Rc, sync::Arc};

/// The one place a prepared partition cursor is admitted: its plans, ordering and proof are checked here.
pub(crate) fn from_prepared<'f>(
    proof: crate::application::file_dag::relations::admission::service::BorrowedInput<'f>,
    partition_plan: Arc<DerivedContract>,
    closure_plan: Arc<DerivedContract>,
    ordered: AcquiredView,
    owner: Rc<RelationOwner>,
) -> Result<PartitionedClosure<'f>, OperatingError> {
    ryo_execution_domain::domain::derived_views::service::validate(
        ryo_derived::domain::request::types::DerivedKind::PartitionOrder,
        &partition_plan,
    )?;
    ryo_execution_domain::domain::derived_views::service::validate(
        ryo_derived::domain::request::types::DerivedKind::ClosureAdjacency,
        &closure_plan,
    )?;
    if !std::ptr::eq(ordered.contract(), partition_plan.as_ref())
        || !matches!(
            ordered.view()?,
            ryo_derived::domain::contract::types::DerivedView::PartitionOrder(_)
        )
    {
        return Err(OperatingError::WrongOperation);
    }
    let input: RepresentationInput<'f> = proof.rows();
    owner
        .derived()
        .admit_partition(&closure_plan, &owner, &proof, &ordered)?;
    #[cfg(test)]
    let ordering_capacity = ordered.partition_order_capacity()?;
    Ok(PartitionedClosure {
        input,
        proof,
        owner,
        partition_plan,
        closure_plan,
        views: PartitionViews {
            ordered,
            current: None,
        },
        #[cfg(test)]
        ordering_capacity,
        cursor: OrderCursor(0),
        key: Vec::new(),
        scratch: DerivedScratch::default(),
    })
}

#[cfg(any(test, feature = "fixtures"))]
impl DerivedOwner {
    /// The partitioned closure `closure_partitions::service::prepare` answers: the publication is admitted and its shared
    /// ordering prepared and acquired.
    pub(crate) fn partitioned_closure<'f>(
        &self,
        publication: &crate::application::file_dag::relations::handle::service::Publication,
        input: RepresentationInput<'f>,
        keys: &[ryo_values::domain::plan_parts::types::ColumnIndex],
        source: ryo_values::domain::plan_parts::types::ColumnIndex,
        destination: ryo_values::domain::plan_parts::types::ColumnIndex,
        owner: Rc<RelationOwner>,
    ) -> Result<PartitionedClosure<'f>, OperatingError> {
        input.validate()?;
        let schema = [
            ryo_execution_domain::domain::operation_schema::types::InputShape {
                kinds: input.kinds(),
                tree: None,
            },
        ];
        let partition_plan = Arc::new(self.prepare_view(
            &ViewRequest::PartitionOrder {
                keys: keys.to_vec(),
            },
            &schema,
        )?);
        let closure_plan = Arc::new(self.prepare_view(
            &ViewRequest::ClosureAdjacency {
                source,
                destination,
            },
            &schema,
        )?);
        let proof = owner.admit(publication.input()?.as_ref(), input)?;
        let admitted = self.admit_view(&partition_plan, &owner, &[&proof])?;
        let ordered = self.acquire_stored(
            &partition_plan,
            &admitted,
            crate::application::file_dag::derived::owner::service::ViewContext::default(),
            DerivedInputs::Rows(&[proof.rows()]),
            &mut DerivedScratch::default(),
            None,
        )?;
        from_prepared(proof, partition_plan, closure_plan, ordered, owner)
    }
}

impl PartitionedClosure<'_> {
    /// Moves to the next partition, building it from the shared ordering, or answers that none is left.
    ///
    /// # Errors
    ///
    /// [`OperatingError::WrongOperation`] when the ordering or the partition plan is of another kind,
    /// [`OperatingError::ShapeMismatch`] when a key cell of an ordered row is missing, and the owner's refusal to admit
    /// or build the next partition.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure;
    /// use ryo_execution_store::application::file_dag::closure_partitions::types::PartitionAdvance;
    /// fn partitions(closure: &mut PartitionedClosure<'_>) -> usize {
    ///     let mut count = 0;
    ///     while let Ok(PartitionAdvance::Advanced) = closure.advance() {
    ///         count += 1;
    ///     }
    ///     count
    /// }
    /// ```
    pub fn advance(&mut self) -> Result<PartitionAdvance, OperatingError> {
        let ryo_derived::domain::contract::types::DerivedView::PartitionOrder(order) =
            self.views.ordered.view()?
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Some(&first) = order.get(self.cursor.0) else {
            self.views.current = None;
            return Ok(PartitionAdvance::Exhausted);
        };
        let ViewRequest::PartitionOrder { keys } = &self.partition_plan.request else {
            return Err(OperatingError::WrongOperation);
        };
        self.key.clear();
        for column in keys {
            self.key.push(
                self.input
                    .cell(first, *column)
                    .ok_or(OperatingError::ShapeMismatch)?,
            );
        }
        let begin = self.cursor.0;
        while let Some(&row) = order.get(self.cursor.0) {
            let same_key = keys
                .iter()
                .zip(&self.key)
                .try_fold(true, |same, (column, value)| {
                    let actual = self
                        .input
                        .cell(row, *column)
                        .ok_or(OperatingError::ShapeMismatch)?;
                    Ok::<_, OperatingError>(same && actual == *value)
                })?;
            if !same_key {
                break;
            }
            self.cursor = OrderCursor(self.cursor.0 + 1);
        }
        // The old partition's mandatory charge ends before its reusable buffer
        // is rebuilt for the next key. A taken partition transfers its lease.
        let admitted = self.owner.derived().admit_partition(
            &self.closure_plan,
            &self.owner,
            &self.proof,
            &self.views.ordered,
        )?;
        self.views.current = Some(self.owner.derived().scoped_build(
            crate::application::file_dag::route_counts::types::BuildRoute::ClosurePartition,
            &self.closure_plan,
            &admitted,
            DerivedInputs::Partition {
                rows: self.input,
                positions: &order[begin..self.cursor.0],
                key: &self.key,
            },
            self.views.current.take(),
            &mut self.scratch,
        )?);
        Ok(PartitionAdvance::Advanced)
    }
    /// Takes the current partition's view out of the cursor, for the invocation to hold.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] when the cursor is on no partition.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure;
    /// use ryo_execution_store::application::file_dag::closure_partitions::types::PartitionAdvance;
    /// use ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
    /// fn first(closure: &mut PartitionedClosure<'_>) -> Option<AcquiredView> {
    ///     match closure.advance() {
    ///         Ok(PartitionAdvance::Advanced) => closure.take_current().ok(),
    ///         _ => None,
    ///     }
    /// }
    /// ```
    pub fn take_current(&mut self) -> Result<AcquiredView, OperatingError> {
        self.views.current.take().ok_or(OperatingError::NotRun)
    }
    /// The partition the cursor is on.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] when the cursor is on no partition, the view's own refusal, and
    /// [`OperatingError::WrongOperation`] when the view is no closure adjacency.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::closure_partitions::service::PartitionedClosure;
    /// fn partitioned(closure: &PartitionedClosure<'_>) -> bool {
    ///     closure.current().is_ok()
    /// }
    /// ```
    pub fn current(&self) -> Result<&ClosurePartition, OperatingError> {
        let current = self.views.current.as_ref().ok_or(OperatingError::NotRun)?;
        let ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(partition) =
            current.view()?
        else {
            return Err(OperatingError::WrongOperation);
        };
        Ok(partition)
    }
}
