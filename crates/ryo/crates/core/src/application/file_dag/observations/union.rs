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
        // The union streams already-owned producer publications. A multi-source
        // input has no single publication identity to lend to a derived view.
        Ok(InputOrigin::Internal)
    }
    fn publication(&self, which: InputPlace) -> Result<Option<PublishedInput>, OperatingError> {
        let sources = self.sources(which)?;
        Ok((sources.len() == 1).then(|| sources[0].clone()))
    }
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError> {
        self.sources(which)?;
        Ok(self.kinds)
    }
    fn streams(&self, which: InputPlace) -> Result<Supply, OperatingError> {
        self.sources(which)?;
        Ok(Supply::Streamed)
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

#[cfg(test)]
mod feed_contract_test {
    use super::*;
    use ryo_relations::domain::relations::types::{Integer, Value};

    #[test]
    fn published_inputs_keep_exact_identity_and_invalid_positions_refuse() {
        let owner = Rc::new(RelationOwner::new(
            true,
            crate::outbound::derived::factory::test_port(),
        ));
        let publications = [1, 2, 3]
            .into_iter()
            .map(|number| {
                owner
                    .computed(Table {
                        kinds: vec![CellKind::Number],
                        cells: vec![Value::Number(Integer(number))],
                        count: RowCount(1),
                    })
                    .expect("valid producer publication")
            })
            .collect::<Vec<_>>();
        let read = |_| Err(OperatingError::NotRead);
        let feed = PublicationUnion {
            owner,
            sources: &publications[..2],
            kinds: &[CellKind::Number],
            read: &read,
        };
        assert!(matches!(
            feed.origin(InputPlace(0)),
            Ok(InputOrigin::Internal)
        ));
        assert!(matches!(
            feed.origin(InputPlace(1)),
            Ok(InputOrigin::Internal)
        ));
        assert!(matches!(feed.streams(InputPlace(0)), Ok(Supply::Streamed)));
        assert!(matches!(feed.streams(InputPlace(1)), Ok(Supply::Streamed)));
        assert!(
            feed.publication(InputPlace(0))
                .unwrap()
                .unwrap()
                .same_record(&publications[0])
        );
        assert!(
            feed.publication(InputPlace(1))
                .unwrap()
                .unwrap()
                .same_record(&publications[1])
        );
        assert!(matches!(
            feed.publication(InputPlace(2)),
            Err(OperatingError::MissingInput { position: 3 })
        ));
        assert!(matches!(
            feed.streams(InputPlace(2)),
            Err(OperatingError::MissingInput { position: 3 })
        ));

        let many = PublicationUnion {
            owner: Rc::clone(&feed.owner),
            sources: &publications[..],
            kinds: &[CellKind::Number],
            read: &read,
        };
        assert!(many.publication(InputPlace(0)).unwrap().is_some());
        assert!(many.publication(InputPlace(1)).unwrap().is_none());
        assert!(matches!(
            many.origin(InputPlace(1)),
            Ok(InputOrigin::Internal)
        ));
        assert!(matches!(many.streams(InputPlace(1)), Ok(Supply::Streamed)));
    }
}
