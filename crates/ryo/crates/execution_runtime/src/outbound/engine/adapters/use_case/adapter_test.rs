//! The `use_case` adapter of the `engine` port: the port's contract rows run through it, and what is its own: the engine's own
//! pack reads back through it, and the engine's refusal of a pack passes through unchanged.

use super::adapter::EngineUseCaseAdapter;
use crate::outbound::engine::fixtures::contract;
use crate::outbound::engine::fixtures::fake::no_kinds;
use crate::outbound::engine::interface::EnginePort as _;
use ryo_engine::domain::packs::service::encoded;
use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
use ryo_engine::inbound::use_cases::project::use_case::ProjectUseCaseImpl;
use ryo_engine::inbound::use_cases::scan::use_case::ScanUseCaseImpl;
use ryo_relations::domain::relations::types::Count;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// The pack the adapter is asked to read.
#[derive(Clone, Copy, Debug)]
enum Pack {
    /// The pack the engine itself writes for no rule.
    EnginesOwn,
    /// A pack whose version is one byte long, `x`: no engine version.
    OtherVersion,
}

type Input = Pack;
/// How many rules the loaded rule set runs.
type Value = Count;
type Failure = LoadError;

/// What is specific to this adapter: the engine's pack and the engine's refusal pass through it unchanged.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_engines_own_pack_reads_back_through_the_adapter"),
        word: Word::Succeeds,
        input: || Pack::EnginesOwn,
        expect: Expect::Value(Count(0)),
    },
    Case {
        name: CaseName("a_pack_of_another_engine_version_is_refused_as_the_engine_said"),
        word: Word::Refuses,
        input: || Pack::OtherVersion,
        expect: Expect::Error(LoadError::PackVersion),
    },
];

/// The adapter over the engine's own use cases.
fn adapter() -> EngineUseCaseAdapter<LoadUseCaseImpl, ScanUseCaseImpl, ProjectUseCaseImpl> {
    EngineUseCaseAdapter::new(
        LoadUseCaseImpl::new(),
        ScanUseCaseImpl::new(),
        ProjectUseCaseImpl::new(),
    )
}

fn subject(pack: Input) -> Result<Value, Failure> {
    let (adapter, digest) = (adapter(), PackDigest([7; 32]));
    contract::check(&adapter);
    let bytes = match pack {
        Pack::EnginesOwn => encoded(&Programs::default(), digest).0,
        Pack::OtherVersion => b"\x01\x00x".to_vec(),
    };
    let programs = adapter.load(LoadSource::Pack(PackBytes(&bytes), digest), &no_kinds())?;
    Ok(Count(adapter.accepted(&programs).len()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
