//! What every `manifest` adapter must pass: the product is found from beneath it, and its tools read in order.

// Error locations are contractual; diagnostic wording belongs to each implementation.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::manifest::types::{Group, Hook, HookName, Manifest, ProductRoot};
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::Products;

/// Where the search starts: at the root, or a missing place.
type Input = Start;
/// The ids of the tools read, joined.
type Value = String;
/// The manifest's error, which a refusal row expects by its location.
type Failure = ManifestError;

/// Where a row starts.
#[derive(Clone, Copy, Debug)]
pub enum Start {
    /// The product's root.
    Root,
    /// A place nothing holds.
    Nowhere,
}

/// The contract's rows: every implementation runs them, over a root holding the tools `fmt` and `lint`.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_tools_are_read_in_order"),
        word: Word::Succeeds,
        input: || Start::Root,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_place_with_no_manifest_above_it_is_refused"),
        word: Word::Refuses,
        input: || Start::Nowhere,
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Missing {start} if std::path::Path::new(start).is_absolute())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Runs the contract's rows against one implementation, whose product is at `root`, whose `nowhere` has none, and
/// whose `blank` is a place a manifest is written to once.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(
    adapter: &impl ManifestPort,
    root: &ProductRoot,
    nowhere: &ProductRoot,
    blank: &ProductRoot,
) {
    let subject = |start: Start| -> Result<Value, Failure> {
        let place = match start {
            Start::Root => root,
            Start::Nowhere => nowhere,
        };
        let answer = adapter.find(place);
        if let Err(ManifestError::Missing { start: searched }) = &answer {
            assert_eq!(
                searched,
                place.0.as_ref(),
                "a missing product names where the search started"
            );
        }
        let found = answer?;
        assert_eq!(
            adapter.members(&found)?,
            Products::default(),
            "a product that declares no members has none"
        );
        assert_eq!(
            adapter.topmost(&found)?,
            found,
            "a lone product is its own topmost"
        );
        // The contract asks only that the hooks read; the table a product without `[hooks]` gets is each
        // implementation's own default.
        drop(adapter.hooks(&found)?);
        let ids = adapter
            .read(&found)?
            .tools
            .iter()
            .map(|tool| tool.id.0.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(if ids == "fmt lint" {
            String::new()
        } else {
            ids
        })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
    let created = Manifest {
        hooks: vec![Hook {
            name: HookName("pre-commit".into()),
            groups: vec![Group("lint".into())],
        }],
        ..Manifest::default()
    };
    assert_eq!(
        adapter.create(blank, &created),
        Ok(()),
        "a manifest is written where none is"
    );
    assert_eq!(
        adapter.read(blank).map(|read| read.hooks),
        Ok(created.hooks),
        "what is written reads back"
    );
}
