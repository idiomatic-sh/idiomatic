//! The steps of the `load` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_engine::domain::packs::constants::VERSION_WIDTH;
use ryo_engine::domain::packs::service::encoded;
use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
use ryo_engine::domain::programs::service::accepted;
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::statistics::types::StatisticsBytes;
use ryo_engine::inbound::use_cases::load::fixtures::{kinds, loaded, loaded_with, said, sampled};
use ryo_engine::inbound::use_cases::load::interface::LoadUseCase as _;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
use ryo_engine::inbound::use_cases::scan::fixtures::scanned;

use crate::Behavior;

/// The scene whose pack another engine version wrote.
const OTHER_VERSION: &str = "other version";

/// The digest the scenarios' packs are written for.
const DIGEST: PackDigest = PackDigest([5; 32]);

/// The programs of the rule set a scene names, loaded whole: what the scan and the project scenarios run. A scene one of
/// whose rules the load refused has no programs to run: its answer is the load's, so a scenario of a rule that is not loaded
/// never passes on what an empty rule set counts.
pub(crate) fn lowered(scene: &str) -> Result<Programs, String> {
    match loaded(scene) {
        Ok(loaded) if loaded.refused.is_empty() => Ok(loaded.programs),
        Ok(loaded) => Err(said(scene, &loaded)),
        Err(error) => Err(error.to_string()),
    }
}

/// Names the scene the next `load` step runs.
#[given(expr = "the load input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Loads the scene's rule set, answering how many rules it runs, what the scene asks of it, and each rule it refused.
#[when(expr = "load runs")]
fn runs(world: &mut Behavior) {
    world.answer = match loaded(&world.input) {
        Ok(loaded) => said(&world.input, &loaded),
        Err(error) => error.to_string(),
    };
}

/// Lowers the scene's rule set, encodes it and loads the pack back, answering how many rules both run or why one refused.
#[when(expr = "load runs through a pack")]
fn runs_through_a_pack(world: &mut Behavior) {
    let programs = if world.input == OTHER_VERSION {
        Ok(Programs::default())
    } else {
        lowered(&world.input)
    };
    world.answer = match programs {
        Ok(programs) => {
            let mut bytes = encoded(&programs, DIGEST).0;
            if world.input == OTHER_VERSION
                && let Some(format) = bytes.get_mut(VERSION_WIDTH.0)
            {
                *format = format.wrapping_add(1);
            }
            match LoadUseCaseImpl.load(LoadSource::Pack(PackBytes(&bytes), DIGEST), &kinds()) {
                Ok(read) if accepted(&read.programs) == accepted(&programs) => {
                    format!(
                        "pack and lowering run {} rules",
                        accepted(&read.programs).len()
                    )
                }
                Ok(read) => format!(
                    "pack runs {:?}, lowering {:?}",
                    accepted(&read.programs),
                    accepted(&programs)
                ),
                Err(error) => error.to_string(),
            }
        }
        Err(refused) => refused,
    };
}

/// Asserts what `load` answered.
#[then(expr = "load answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}

/// What a load of the scene's rule set with `statistics` finds in a file of `text`: each row as text, sorted.
fn found(scene: &str, statistics: &[u8], text: &str) -> Vec<String> {
    let programs = loaded_with(scene, StatisticsBytes(statistics)).map(|loaded| loaded.programs);
    let mut rows = match programs.map(|programs| scanned(&programs, text)) {
        Ok(Ok(Some(scan))) => scan.rows,
        Ok(Ok(None) | Err(_)) | Err(_) => Vec::new(),
    };
    rows.sort();
    rows
}

/// Loads the scene's rule set with no statistics and with the statistics of a sample of it, scans the scenario's file with
/// each, and asserts that the two found the same rows and that there were rows to find.
#[then(expr = "both programs found the same rows")]
fn found_the_same(world: &mut Behavior) {
    let Ok(statistics) = sampled(&world.input) else {
        panic!("the sample's rule set was not loaded");
    };
    let without = found(&world.input, &[], &world.text);
    let with = found(&world.input, &statistics.0, &world.text);
    assert!(
        !without.is_empty(),
        "the rule set found no row: nothing is compared"
    );
    assert_eq!(without, with);
}
