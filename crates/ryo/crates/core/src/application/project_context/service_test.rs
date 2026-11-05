//! Warm project-entry observation controls over the shared corpus port.
use super::as_it_was;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::digest::fixtures::fake::DigestFake;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::project_context::types::{EntryObservation, EntryState};
use ryo_values::domain::files::types::{CorpusFile, FileBytes};
use ryo_values::domain::records::types::Freshness;

fn file(path: &str) -> CorpusFile {
    CorpusFile {
        executable: Default::default(),
        path: FilePath(path.into()),
        bytes: FileBytes(b"pass\n".to_vec()),
    }
}
fn observation(path: &str, state: EntryState) -> EntryObservation {
    EntryObservation {
        path: FilePath(path.into()),
        state,
    }
}
#[test]
fn warm_context_reobserves_add_remove_and_uncertainty() {
    let absent = observation("local.py", EntryState::Missing);
    let present = observation("local.py", EntryState::File);
    let unknown = observation("local.py", EntryState::Unknown);
    let empty = CorpusFake::default();
    let populated = CorpusFake {
        files: vec![file("local.py")],
        ..CorpusFake::default()
    };
    let check = |corpus: &CorpusFake, entry: &EntryObservation| {
        as_it_was(corpus, &DigestFake, &[], &[], std::slice::from_ref(entry))
    };
    assert_eq!(check(&empty, &absent), Freshness::AsItWas);
    assert_eq!(check(&populated, &absent), Freshness::Changed);
    assert_eq!(check(&populated, &present), Freshness::AsItWas);
    assert_eq!(check(&empty, &present), Freshness::Changed);
    assert_eq!(check(&empty, &unknown), Freshness::Changed);
}

#[test]
fn context_identity_binds_ordered_roots_and_given_inventory() {
    use super::identity;
    use ryo_language::domain::project_context::types::{
        GivenProjectInventory, ProjectInventoryCompleteness, ProjectSearchScope,
    };
    use ryo_language::inbound::use_cases::project::types::{Repository, RootPath};
    use ryo_values::domain::runs::types::Session;
    let mut session = Session {
        root: RootPath(".".into()),
        repository: Repository::Given,
        search_scope: ProjectSearchScope::Declared(vec![
            FilePath("src".into()),
            FilePath("vendor".into()),
        ]),
        given_inventory: GivenProjectInventory::default(),
    };
    let original = identity(&[], &session, &DigestFake).expect("identity");
    if let ProjectSearchScope::Declared(roots) = &mut session.search_scope {
        roots.reverse();
    }
    assert_ne!(
        original,
        identity(&[], &session, &DigestFake).expect("reordered roots")
    );
    session.given_inventory.completeness =
        ProjectInventoryCompleteness::CompleteWithinDeclaredScope;
    let complete = identity(&[], &session, &DigestFake).expect("complete inventory");
    assert_ne!(complete, original);
    session
        .given_inventory
        .entries
        .push(observation("pkg", EntryState::Directory));
    assert_ne!(
        complete,
        identity(&[], &session, &DigestFake).expect("changed entry")
    );
}
