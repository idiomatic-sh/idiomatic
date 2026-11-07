//! The thread adapter of the profile port: the contract rows run through it, then the cases specific to it.

use super::adapter::ProfileThreadAdapter;
use super::constants::THREAD_PROFILES;
use super::types::ThreadProfiles;
use crate::outbound::profile::fixtures::contract::check;
use crate::outbound::profile::interface::ProfilePort as _;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use thread_local::ThreadLocal;

/// A question this adapter is asked beyond the contract.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// A profile kept on one thread is looked for on another.
    OtherThread,
    /// The thread profiles are made with room for the workers.
    Room,
}

type Input = Question;
type Value = bool;
type Failure = ();

/// The rows of what is specific to this adapter: a name, one case word, an input and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_profile_kept_on_one_thread_is_empty_on_another"),
        word: Word::Succeeds,
        input: || Question::OtherThread,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_thread_profiles_are_made_with_room_for_the_workers"),
        word: Word::Succeeds,
        input: || Question::Room,
        expect: Expect::Value(true),
    },
];

/// Calls the adapter on one input, for its own rows.
fn subject(input: Input) -> Value {
    match input {
        Question::OtherThread => {
            let adapter = std::sync::Arc::new(ProfileThreadAdapter::<u8>::new());
            adapter.with_profile(&mut |profile| *profile = Some(1));
            let other = std::sync::Arc::clone(&adapter);
            std::thread::spawn(move || {
                let mut empty = false;
                other.with_profile(&mut |profile| empty = profile.is_none());
                empty
            })
            .join()
            .unwrap_or(false)
        }
        Question::Room => {
            let profiles = ThreadProfiles::<u8>(ThreadLocal::with_capacity(THREAD_PROFILES));
            profiles.0.iter().count() == 0 && THREAD_PROFILES > 0
        }
    }
}

#[test]
fn succeeds() {
    check(&ProfileThreadAdapter::<u8>::new());
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
