//! The metadata feature calls the public pure preflight use case.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_execution::inbound::use_cases::metadata::{
    interface::MetadataUseCase,
    types::{AffectedFiles, CoverageFileCount},
    use_case::MetadataUseCaseImpl,
};
use ryo_relations::domain::relations::types::FileId;

#[given(expr = "a metadata request {string}")]
fn request(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "metadata runs")]
fn runs(world: &mut Behavior) {
    let metadata = MetadataUseCaseImpl;
    world.answer = if world.input == "foreign file" {
        let affected = [Some(FileId(99))];
        assert!(
            metadata
                .affected_ordinals(AffectedFiles {
                    affected: &affected,
                    file_count: CoverageFileCount(1),
                })
                .is_err()
        );
        "refused"
    } else {
        assert!(!metadata.feature_catalog().is_empty());
        "catalogued"
    }
    .to_owned();
}

#[then(expr = "metadata answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
