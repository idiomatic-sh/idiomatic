//! What every `engine` adapter must pass: the empty rule set loads and runs no rule, its pack reads back for the digest it was
//! written for and for no other, and a rule set with no rule finds no row, in a file or across files.

use crate::outbound::engine::fixtures::fake::{Counted, no_kinds, visiting};
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::lowering::types::SelectedRules;
use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_engine::inbound::use_cases::project::errors::ProjectPassError;
use ryo_engine::inbound::use_cases::project::fixtures::{NoProject, OneFile};
use ryo_engine::inbound::use_cases::scan::errors::ScanError;
use ryo_language::domain::facts::types::{FileFacts, ProjectFacts};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::node_table::service::builder;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::errors::WalkError;
use ryo_language::inbound::use_cases::walk::interface::WalkUseCase;
use ryo_language::inbound::use_cases::walk::types::{RelationNeeds, WalkRequest};
use ryo_relations::domain::relations::types::{Count, FileId};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: which question to ask of the adapter.
type Input = Question;
/// What a row of the contract expects when it succeeds: how many rules the rule set runs, or how many rows it found.
type Value = Count;
/// What a row of the contract expects when it is refused.
type Failure = ContractFault;

/// A question the contract asks of an adapter.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// The rule set of no selected rule loads and runs no rule.
    NoRuleSelected,
    /// The pack written for a rule set reads back the rules it runs.
    PackReadBack,
    /// A rule set with no rule finds no row in a file.
    NothingFoundInAFile,
    /// A rule set with no rule finds no row across files.
    NothingFoundAcrossFiles,
    /// A pack is asked for under another digest than the one it was written for.
    PackOfOtherRules,
    /// Bytes that are no pack are asked for.
    NoPack,
}

/// Where an answer of the port stopped.
#[derive(Debug, PartialEq, Eq)]
pub enum ContractFault {
    /// `load` refused.
    Load(LoadError),
    /// The fixture language refused the file.
    Walk(WalkError),
    /// The fixture language lent no file.
    NotLent,
    /// `scan` faulted.
    Scan(ScanError),
    /// `project` faulted.
    Project(ProjectPassError),
}

/// The contract's rows: a name, one case word, an input and the expected value or error. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_rule_set_of_no_selected_rule_runs_no_rule"),
        word: Word::Succeeds,
        input: || Question::NoRuleSelected,
        expect: Expect::Value(Count(0)),
    },
    Case {
        name: CaseName("a_pack_reads_back_the_rules_it_was_written_for"),
        word: Word::Succeeds,
        input: || Question::PackReadBack,
        expect: Expect::Value(Count(0)),
    },
    Case {
        name: CaseName("a_rule_set_with_no_rule_finds_no_row_in_a_file"),
        word: Word::Succeeds,
        input: || Question::NothingFoundInAFile,
        expect: Expect::Value(Count(0)),
    },
    Case {
        name: CaseName("a_rule_set_with_no_rule_finds_no_row_across_files"),
        word: Word::Succeeds,
        input: || Question::NothingFoundAcrossFiles,
        expect: Expect::Value(Count(0)),
    },
    Case {
        name: CaseName("a_pack_written_for_other_rules_is_refused"),
        word: Word::Refuses,
        input: || Question::PackOfOtherRules,
        expect: Expect::Error(ContractFault::Load(LoadError::PackDigest)),
    },
    Case {
        name: CaseName("bytes_that_are_no_pack_are_refused"),
        word: Word::Refuses,
        input: || Question::NoPack,
        expect: Expect::Error(ContractFault::Load(LoadError::PackCorrupt)),
    },
];

/// How many rows a rule set finds through `adapter` in a file of `text`, walked by the fixture language.
///
/// # Errors
///
/// The fixture language's refusal of the file, or the scan's fault.
pub fn found_in(
    adapter: &dyn EnginePort,
    programs: &Programs,
    text: &str,
) -> Result<Count, ContractFault> {
    let request = WalkRequest {
        path: SourcePath("scene.walk"),
        file: FileId(1),
        source: ByteSlice(text.as_bytes()),
    };
    let (mut table, mut answer) = (NodeTable::default(), None);
    let mut scanning = visiting(|file: FileFacts<'_>| {
        let mut found = Counted(Count(0));
        let state = &mut adapter.state_for(programs);
        answer = Some(
            adapter
                .scan(programs, file, state, &mut found)
                .map(|_| found.0),
        );
    });
    Walking
        .walk(
            request,
            RelationNeeds(&[]),
            &mut builder(&mut table, <Walking as WalkUseCase>::KIND_COUNT),
            &mut (),
            &mut scanning,
        )
        .map_err(ContractFault::Walk)?;
    match answer {
        Some(answer) => answer.map_err(ContractFault::Scan),
        None => Err(ContractFault::NotLent),
    }
}

/// Asks one question of the adapter.
fn answer(adapter: &dyn EnginePort, question: Question) -> Result<Value, Failure> {
    let (kinds, digest) = (no_kinds(), PackDigest([7; 32]));
    let nothing = || {
        adapter
            .load(LoadSource::Plans(SelectedRules(&[])), &kinds)
            .map_err(ContractFault::Load)
    };
    let read_back = |bytes: &[u8], digest| {
        adapter
            .load(LoadSource::Pack(PackBytes(bytes), digest), &kinds)
            .map(|programs| Count(adapter.accepted(&programs).len()))
            .map_err(ContractFault::Load)
    };
    match question {
        Question::NoRuleSelected => Ok(Count(adapter.accepted(&nothing()?).len())),
        Question::PackReadBack => read_back(&adapter.encoded(&nothing()?, digest).0, digest),
        Question::PackOfOtherRules => {
            read_back(&adapter.encoded(&nothing()?, digest).0, PackDigest([8; 32]))
        }
        Question::NoPack => read_back(b"no", digest),
        Question::NothingFoundInAFile => found_in(adapter, &nothing()?, "one two"),
        Question::NothingFoundAcrossFiles => {
            let (rules, mut found) = (nothing()?, Counted(Count(0)));
            let scratch = &mut adapter.state_for(&rules);
            let facts = ProjectFacts {
                facts: &mut NoProject,
            };
            adapter
                .project(&rules, &OneFile { empty: true }, facts, scratch, &mut found)
                .map(|_| found.0)
                .map_err(ContractFault::Project)
        }
    }
}

/// Runs the contract's rows against one implementation: the fake, or the adapter over the engine's use cases.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &dyn EnginePort) {
    run(CASES, Word::Succeeds, |question| answer(adapter, question));
    run(CASES, Word::Refuses, |question| answer(adapter, question));
}
