//! Generation checks, identity matches, resolution and physical inspection of publications.
use crate::application::file_dag::relations::types::NormalizationCharge;
use crate::application::file_dag::relations::*;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
impl RelationOwner {
    pub(crate) fn publication_kinds<'a>(
        &self,
        publication: &'a PublishedInput,
    ) -> Result<&'a [CellKind], OperatingError> {
        let record = publication.as_ref();
        self.generation(record)?;
        Ok(match &record.body {
            RelationBody::Source(source) => &source.kinds,
            RelationBody::Computed(table) => &table.kinds,
            RelationBody::Compact(table) => {
                ryo_values::domain::compact_table::service::compact_kinds(table)
            }
        })
    }
    pub(crate) fn private<'a>(
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

    pub(crate) fn generation(&self, result: &RelationResult) -> Result<(), OperatingError> {
        if Arc::ptr_eq(&self.generation.borrow(), &result.generation) {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(crate) fn matches_current(&self, result: &RelationResult, input: RepresentationInput<'_>) -> Sameness {
        #[cfg(test)]
        self.identity_checks
            .set(CheckCount(self.identity_checks.get().0 + 1));
        // publish_selection verifies its source epoch; private fields preserve
        // that invariant, so the selected chain shares the checked outer epoch.
        if result.normalized.get().is_some_and(|table| {
            matches!(input, RepresentationInput::Computed(actual) if std::ptr::eq(actual, table.as_ref()))
        }) {
            return Sameness::Same;
        }
        match &result.body {
            RelationBody::Source(source) => identity::matches(source, input),
            RelationBody::Computed(table) => {
                if matches!(input, RepresentationInput::Computed(actual) if std::ptr::eq(actual, table.as_ref()))
                {
                    Sameness::Same
                } else {
                    Sameness::Different
                }
            }
            RelationBody::Compact(table) => {
                if matches!(input, RepresentationInput::Compact(actual) if std::ptr::eq(actual, table.as_ref()))
                {
                    Sameness::Same
                } else {
                    Sameness::Different
                }
            }
        }
    }
    pub(crate) fn resolve<'r, 'f>(
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
                if identity::matches(source, RepresentationInput::from(&rows)) == Sameness::Different {
                    return Err(OperatingError::NotRead);
                }
                Ok(rows)
            }
            RelationBody::Computed(table) => Ok(Rows::Computed(table)),
            RelationBody::Compact(table) => Ok(Rows::Compact(table)),
        }
    }
    /// The caller invokes this only at its already declared physical demand gate.
    pub(crate) fn inspect(
        &self,
        result: &dyn PublishedRelation,
        input: RepresentationInput<'_>,
    ) -> Result<PhysicalShape, OperatingError> {
        let result = self.private(result)?;
        if self.matches_current(result, input) == Sameness::Different {
            return Err(OperatingError::NotRead);
        }
        self.inspect_current(result, input)
    }
    pub(crate) fn inspect_current(
        &self,
        result: &RelationResult,
        input: RepresentationInput<'_>,
    ) -> Result<PhysicalShape, OperatingError> {
        if result.normalized.get().is_some_and(|table|matches!(input,RepresentationInput::Computed(actual)if std::ptr::eq(actual,table.as_ref()))){self.reuses.set(CheckCount(self.reuses.get().0+1));return Ok(PhysicalShape::Complete)}
        if result.inspected.get().is_some() {
            self.reuses.set(CheckCount(self.reuses.get().0 + 1));
        }
        let outcome = result
            .inspected
            .get_or_init(|| {
                self.checks.set(CheckCount(self.checks.get().0 + 1));
                let contract = input.contract()?;
                crate::application::file_dag::route_counts::observe(
                    &self.routes,
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
                        .set(crate::application::file_dag::relations::types::PinnedViewCount(
                            self.views.get().0.saturating_sub(before - entry.views.len()),
                        ));
                }
            }
        }
        outcome
    }
    pub(crate) fn normalize<'r, 'f>(
        &self,
        publication: &'r PublishedInput,
        rows: Rows<'f>,
    ) -> Result<Rows<'r>, OperatingError>
    where
        'f: 'r,
    {
        let result = self.private(publication.as_ref())?;
        match self.inspect(result, RepresentationInput::from(&rows))? {
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
