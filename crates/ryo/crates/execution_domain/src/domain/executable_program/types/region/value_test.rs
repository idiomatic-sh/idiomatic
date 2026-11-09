//! A region starts on the generic route with no resolved dependencies, and only the compiler's writes change its readers,
//! route, dependencies and source partitions.
use crate::domain::operator::types::ReaderMultiplicity;

use super::value::Region;
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{InputSource, RegionId, Route, SourcePartition},
    source_partitions::types::{PartitionRequirement},
    operating::errors::OperatingError,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepScope;

fn region() -> Region {
    Region::new(
        RuleIndex(1),
        Ok(Box::new([])),
        StepScope::CrossFile,
        Box::new([]),
        Box::new([InputSource::Relation(Relation::Function)]),
        ReaderMultiplicity(1),
    )
}

#[test]
fn a_new_region_is_generic_with_no_resolved_dependencies() {
    let region = region();
    assert_eq!(region.producer(), RuleIndex(1));
    assert_eq!(region.phase(), StepScope::CrossFile);
    assert_eq!(region.inputs(), [InputSource::Relation(Relation::Function)]);
    assert_eq!(region.consumers(), ReaderMultiplicity(1));
    assert!(matches!(region.route(), Route::Generic));
    assert!(region.dependencies().is_empty());
    assert!(region.source_partitions().is_empty());
    assert!(region.needs().as_ref().is_ok_and(|needs| needs.is_empty()));
    assert!(matches!(
        region.last_operation(),
        Err(OperatingError::NotRun)
    ));
}

#[test]
fn the_compiler_writes_readers_dependencies_and_partitions() {
    let mut region = region();
    region.add_consumer();
    assert_eq!(region.consumers(), ReaderMultiplicity(2));
    region.set_consumers(ReaderMultiplicity(5));
    assert_eq!(region.consumers(), ReaderMultiplicity(5));
    region.set_dependencies(Box::new([RegionId(0), RegionId(0)]));
    assert_eq!(region.dependencies(), [RegionId(0), RegionId(0)]);
    let declared = SourcePartition {
        source: InputSource::Relation(Relation::Function),
        requirement: PartitionRequirement::CurrentFile,
    };
    region.set_source_partitions(Box::new([declared]));
    assert_eq!(region.source_partitions().len(), 1);
    assert_eq!(
        region.source_partitions()[0].requirement,
        PartitionRequirement::CurrentFile
    );
}
