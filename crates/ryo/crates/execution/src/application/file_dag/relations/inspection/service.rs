//! Generation checks, identity matches, resolution and physical inspection of publications.
use crate::application::file_dag::relations::types::NormalizationCharge;
use crate::application::file_dag::relations::*;
impl RelationOwner {
    pub(in crate::application::file_dag) fn publication_kinds<'a>(
        &self,
        publication: &'a PublishedInput,
    ) -> Result<&'a [CellKind], OperatingError> {
        let record = publication.as_ref();
        self.generation(record)?;
        Ok(match &record.body {
            RelationBody::Source(source) => &source.kinds,
            RelationBody::Selected { selection, .. } => &selection.kinds,
            RelationBody::Positioned { kinds, .. } => kinds,
            RelationBody::Computed(table) => &table.kinds,
            RelationBody::Compact(table) => {
                ryo_values::domain::compact_table::service::compact_kinds(table)
            }
        })
    }
    pub(in crate::application::file_dag::relations) fn private<'a>(
        &self,
        result: &'a dyn PublishedRelation,
    ) -> Result<&'a RelationResult, OperatingError> {
        let record = result
            .as_any()
            .downcast_ref::<RelationResult>()
            .ok_or(OperatingError::NotRead)?;
        self.generation(record)?;
        Ok(record)
    }

    pub(in crate::application::file_dag::relations) fn generation(&self, result: &RelationResult) -> Result<(), OperatingError> {
        if Arc::ptr_eq(&self.generation.borrow(), &result.generation) {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(in crate::application::file_dag::relations) fn matches_current(&self, result: &RelationResult, input: BorrowedRows<'_>) -> Sameness {
        #[cfg(test)]
        self.identity_checks
            .set(CheckCount(self.identity_checks.get().0 + 1));
        // publish_selection verifies its source epoch; private fields preserve
        // that invariant, so the selected chain shares the checked outer epoch.
        if result.normalized.get().is_some_and(|table| {
            matches!(input, BorrowedRows::Computed(actual) if std::ptr::eq(actual, table.as_ref()))
        }) {
            return Sameness::Same;
        }
        match &result.body {
            RelationBody::Source(source) => identity::matches(source, input),
            RelationBody::Computed(table) => {
                if matches!(input, BorrowedRows::Computed(actual) if std::ptr::eq(actual, table.as_ref()))
                {
                    Sameness::Same
                } else {
                    Sameness::Different
                }
            }
            RelationBody::Compact(table) => {
                if matches!(input, BorrowedRows::Compact(actual) if std::ptr::eq(actual, table.as_ref()))
                {
                    Sameness::Same
                } else {
                    Sameness::Different
                }
            }
            RelationBody::Positioned {
                source,
                positions,
                shape,
                kinds,
            } => match input {
                BorrowedRows::Positioned(actual)
                    if std::ptr::eq(actual.positions, positions.as_ref())
                        && std::ptr::eq(actual.shape, shape.as_ref())
                        && actual.kinds == kinds.as_ref() =>
                {
                    self.matches_current(source, BorrowedRows::Tree(actual.tree))
                }
                _ => Sameness::Different,
            },
            RelationBody::Selected { selection, source } => match input {
                BorrowedRows::Selected {
                    source: actual,
                    positions,
                    columns,
                    kinds,
                } if std::ptr::eq(positions, selection.rows.as_slice())
                    && columns == selection.columns
                    && kinds == selection.kinds =>
                {
                    self.matches_current(source, BorrowedRows::Stored(actual))
                }
                _ => Sameness::Different,
            },
        }
    }
    pub(in crate::application::file_dag) fn resolve<'r, 'f>(
        &self,
        result: &'r dyn PublishedRelation,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    ) -> Result<Rows<'r>, OperatingError>
    where
        'f: 'r,
    {
        let result = self.private(result)?;
        match &result.body {
            RelationBody::Source(source) => {
                let rows = read(source.relation)?;
                if identity::matches(source, BorrowedRows::from(&rows)) == Sameness::Different {
                    return Err(OperatingError::NotRead);
                }
                Ok(rows)
            }
            RelationBody::Computed(table) => Ok(Rows::Computed(table)),
            RelationBody::Compact(table) => Ok(Rows::Compact(table)),
            RelationBody::Positioned {
                source,
                positions,
                shape,
                kinds,
            } => {
                let Rows::Tree(tree) = self.resolve(source.as_ref(), read)? else {
                    return Err(OperatingError::WrongOperation);
                };
                Ok(Rows::Positioned(
                    crate::domain::table_model::types::PositionedRows {
                        tree,
                        positions,
                        shape,
                        kinds,
                    },
                ))
            }
            RelationBody::Selected { source, selection } => {
                let Rows::Stored(columns) = self.resolve(source.as_ref(), read)? else {
                    return Err(OperatingError::WrongOperation);
                };
                Ok(Rows::Selected(SelectedRows {
                    source: columns,
                    selection: Arc::clone(selection),
                }))
            }
        }
    }
    /// The caller invokes this only at its already declared physical demand gate.
    pub(in crate::application::file_dag) fn inspect(
        &self,
        result: &dyn PublishedRelation,
        input: BorrowedRows<'_>,
    ) -> Result<PhysicalShape, OperatingError> {
        let result = self.private(result)?;
        if self.matches_current(result, input) == Sameness::Different {
            return Err(OperatingError::NotRead);
        }
        self.inspect_current(result, input)
    }
    pub(in crate::application::file_dag::relations) fn inspect_current(
        &self,
        result: &RelationResult,
        input: BorrowedRows<'_>,
    ) -> Result<PhysicalShape, OperatingError> {
        if result.normalized.get().is_some_and(|table|matches!(input,BorrowedRows::Computed(actual)if std::ptr::eq(actual,table.as_ref()))){self.reuses.set(CheckCount(self.reuses.get().0+1));return Ok(PhysicalShape::Complete)}
        if result.inspected.get().is_some() {
            self.reuses.set(CheckCount(self.reuses.get().0 + 1));
        }
        let outcome = result
            .inspected
            .get_or_init(|| {
                self.checks.set(CheckCount(self.checks.get().0 + 1));
                let contract = input.contract()?;
                crate::application::file_dag::route_counts::observe(
                    super::constants::REPRESENTATION_INSPECT,
                    crate::application::file_dag::route_counts::representation(contract.kind),
                    || registered(contract.kind).inspect(&contract),
                )
            })
            .clone();
        if outcome != Ok(PhysicalShape::Complete) {
            if let RelationBody::Source(source) = &result.body {
                if let Some(entry) = self.sources.borrow_mut().get_mut(&source.relation) {
                    let before = entry.views.len();
                    entry
                        .views
                        .retain(|_, view| !std::ptr::eq(view.as_ref(), result));
                    self.views
                        .set(self.views.get().saturating_sub(before - entry.views.len()));
                }
            }
        }
        outcome
    }
    pub(in crate::application::file_dag) fn normalize<'r, 'f>(
        &self,
        publication: &'r PublishedInput,
        rows: Rows<'f>,
    ) -> Result<Rows<'r>, OperatingError>
    where
        'f: 'r,
    {
        let result = self.private(publication.as_ref())?;
        match self.inspect(result, BorrowedRows::from(&rows))? {
            PhysicalShape::Complete => Ok(rows),
            PhysicalShape::ReadablePhysicalFallback => {
                let table = result.normalized.get_or_init(|| {
                    let mut table = Table::default();
                    materialize(&rows, &mut table, &mut Vec::new());
                    Arc::new(table)
                });
                if publication.normalization_charged() == NormalizationCharge::Uncharged {
                    let lease = self.derived.required_lease(&[Self::table_backing(table)])?;
                    publication.hold_normalization(lease);
                }
                Ok(Rows::Computed(table))
            }
        }
    }
}
