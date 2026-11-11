//! S15: through the store port, a publication is readable while its generation lives and refused once the store releases it.
use crate::boundary_fixture::fixtures::source;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_store::application::file_dag::relations::interface::RelationStore;
use ryo_execution_store::application::file_dag::relations::publication::types::{
    ProducerScope, PublicationBody,
};
use ryo_execution_store::application::file_dag::relations::types::ProducerId;

#[test]
fn a_publication_is_read_while_its_generation_lives_and_refused_after_release() {
    let owner = super::super::fixtures::store();
    let publication = owner
        .publish(PublicationBody::Computed(source()))
        .expect("the store publishes a computed table");
    assert_eq!(owner.kinds(&publication).map(<[_]>::len), Ok(4));
    owner
        .bind_producer(ProducerScope::File, ProducerId(0), &publication)
        .expect("a producer binds its publication");
    owner.retire_producer(ProducerScope::File, ProducerId(0));
    assert_eq!(
        owner.kinds(&publication).map(<[_]>::len),
        Ok(4),
        "retiring the producer's derived views leaves its publication readable"
    );
    owner.release();
    assert_eq!(
        owner.kinds(&publication).map(<[_]>::len),
        Err(OperatingError::NotRead),
        "the released generation refuses every publication of it"
    );
    let later = owner
        .publish(PublicationBody::Computed(source()))
        .expect("a later generation publishes again");
    assert_eq!(owner.kinds(&later).map(<[_]>::len), Ok(4));
}
