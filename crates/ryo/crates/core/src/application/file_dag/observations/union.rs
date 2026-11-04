//! N-ary publication delivery through one registered Union invocation and its one distinct workspace.
use crate::application::file_dag::{
    executor::ScopedExecutor,
    feed::{Feed, InputOrigin},
    relations::{PublishedInput, RelationOwner},
};
use crate::domain::{
    operating::{
        errors::OperatingError,
        types::{Emit, InputPlace, OperatorContext, Room, Supply},
    },
    operator::contract::types::OperatorContract,
    replay::{service::scan, types::Collect},
    table_model::{
        service::{kinds_of, reset},
        types::Rows,
    },
};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use std::rc::Rc;

pub(super) fn combine<'a, 'f: 'a>(
    sources: &'a [PublishedInput],
    kinds: &'a [CellKind],
    contract: &OperatorContract,
    read: &'a dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    executor: &ScopedExecutor<'_>,
    context: &mut OperatorContext<'_>,
) -> Result<PublishedInput, OperatingError> {
    let owner = Rc::clone(executor.owner());
    for source in sources {
        let rows = owner.resolve(source.as_ref(), read)?;
        if kinds_of(&rows) != kinds {
            return Err(OperatingError::ObservationIncomplete {
                reason: "producer publication differs from its bound schema".into(),
            });
        }
    }
    let mut feed = PublicationUnion {
        owner: Rc::clone(&owner),
        sources,
        kinds,
        read,
    };
    let mut table = Table::default();
    reset(&mut table, kinds);
    executor.stream(
        contract,
        &mut feed,
        context,
        &mut Room::default(),
        &mut Collect(&mut table),
    )?;
    owner.computed(table)
}
/// Two logical streamed inputs preserve the existing binary Union contract:
/// first publication, then the remaining publications. No accumulated prefix is copied or rehashed.
struct PublicationUnion<'a, 'f> {
    owner: Rc<RelationOwner>,
    sources: &'a [PublishedInput],
    kinds: &'a [CellKind],
    read: &'a dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
}
impl PublicationUnion<'_, '_> {
    fn sources(&self, which: InputPlace) -> Result<&[PublishedInput], OperatingError> {
        match which.0 {
            0 => self.sources.get(..self.sources.len().min(1)),
            1 => self.sources.get(self.sources.len().min(1)..),
            _ => None,
        }
        .ok_or(OperatingError::MissingInput {
            position: which.0 + 1,
        })
    }
}
impl<'a, 'f: 'a> Feed<'a> for PublicationUnion<'a, 'f> {
    fn relation_owner(&self) -> Rc<RelationOwner> {
        Rc::clone(&self.owner)
    }
    fn origin(&self, which: InputPlace) -> Result<InputOrigin, OperatingError> {
        self.sources(which)?;
        Ok(InputOrigin::External)
    }
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError> {
        self.sources(which)?;
        Ok(self.kinds)
    }
    fn streams(&self, _: InputPlace) -> Supply {
        Supply::Streamed
    }
    fn found(&mut self, _: InputPlace, _: RowCount) {}
    fn stream(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let mut row = Vec::new();
        for publication in self.sources(which)? {
            let rows = self.owner.resolve(publication.as_ref(), self.read)?;
            if kinds_of(&rows) != self.kinds {
                return Err(OperatingError::ObservationIncomplete {
                    reason: "producer publication differs from its bound schema".into(),
                });
            }
            scan(&rows, &mut row, context, emit)?;
        }
        Ok(())
    }
    fn hold<'t>(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        into: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'a: 't,
    {
        reset(into, self.kinds);
        self.stream(which, context, &mut Collect(into))?;
        Ok(Rows::Computed(into))
    }
}

use ryo_values::domain::rows::types::{RowCount, Table};
