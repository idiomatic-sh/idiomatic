//! What every `project` adapter must pass.

use super::super::errors::ManifestReadingError;
use super::super::interface::ManifestReadingPort;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::modules::types::Manifestness;
use ryo::languages::ManifestText;

/// What a row of the contract is given: the text of a `go.mod`.
type Input = &'static str;
/// The path of the module the manifest declares, as written; empty for none.
type Value = &'static str;
/// What a row expects when the manifest cannot be read.
type Failure = ManifestReadingError;

/// Whether the manifest is refused for naming no module.
const fn no_module(error: &ManifestReadingError) -> Selection {
    match error {
        ManifestReadingError::NoModule => Selection::Matches,
        ManifestReadingError::Unopened { .. }
        | ManifestReadingError::Unclosed { .. }
        | ManifestReadingError::Incomplete { .. }
        | ManifestReadingError::Unquoted { .. } => Selection::Differs,
    }
}

/// Whether the manifest is refused for a parenthesis that closes no block.
const fn unopened(error: &ManifestReadingError) -> Selection {
    match error {
        ManifestReadingError::Unopened { .. } => Selection::Matches,
        ManifestReadingError::NoModule
        | ManifestReadingError::Unclosed { .. }
        | ManifestReadingError::Incomplete { .. }
        | ManifestReadingError::Unquoted { .. } => Selection::Differs,
    }
}

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_manifest_with_a_module_directive_names_the_module"),
        word: Word::Succeeds,
        input: || "module example.com/app\n",
        expect: Expect::Value("example.com/app"),
    },
    Case {
        name: CaseName("a_manifest_with_no_module_directive_is_refused"),
        word: Word::Refuses,
        input: || "go 1.24\n",
        expect: Expect::Refusal(no_module),
    },
    Case {
        name: CaseName("a_closing_parenthesis_with_no_block_is_refused"),
        word: Word::Refuses,
        input: || "module m\n)\n",
        expect: Expect::Refusal(unopened),
    },
];

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl ManifestReadingPort) {
    let subject = |text: Input| -> Result<Value, Failure> {
        let declared = adapter.declared(Manifestness::Module, &ManifestText(text.into()))?;
        Ok(declared.module.map_or("", |module| Box::leak(module.0)))
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
