//! Warm project-entry observation controls over the shared corpus port.
use super::service::as_it_was;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::digest::fixtures::fake::DigestFake;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::project_context::types::{EntryObservation, ObservedEntryState};
use ryo_values::domain::files::types::{CorpusFile, FileBytes};
use ryo_values::domain::records::types::Freshness;

fn file(path: &str) -> CorpusFile {
    CorpusFile {
        executable: Default::default(),
        path: FilePath(path.into()),
        bytes: FileBytes(b"pass\n".to_vec()),
    }
}
fn observation(path: &str, state: ObservedEntryState) -> EntryObservation {
    EntryObservation {
        path: FilePath(path.into()),
        state,
    }
}
/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A warm context reobserves an added entry, a removed entry and an uncertain one.
    WarmReobserves,
    /// The identity binds the ordered roots and the given inventory.
    IdentityBinds,
}
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;

fn warm_context_reobserves_add_remove_and_uncertainty() -> Result<Value, Failure> {
    let absent = observation("local.py", ObservedEntryState::Missing);
    let present = observation("local.py", ObservedEntryState::File);
    let unknown = observation("local.py", ObservedEntryState::Unknown);
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
    Ok(())
}

fn context_identity_binds_ordered_roots_and_given_inventory() -> Result<Value, Failure> {
    use super::service::identity;
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
        .push(observation("pkg", ObservedEntryState::Directory));
    assert_ne!(
        complete,
        identity(&[], &session, &DigestFake).expect("changed entry")
    );
    Ok(())
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::WarmReobserves => warm_context_reobserves_add_remove_and_uncertainty(),
        Scenario::IdentityBinds => context_identity_binds_ordered_roots_and_given_inventory(),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_warm_context_reobserves_an_added_a_removed_and_an_uncertain_entry"),
        word: Word::Succeeds,
        input: || Scenario::WarmReobserves,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the_identity_binds_the_ordered_roots_and_the_given_inventory"),
        word: Word::Succeeds,
        input: || Scenario::IdentityBinds,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
