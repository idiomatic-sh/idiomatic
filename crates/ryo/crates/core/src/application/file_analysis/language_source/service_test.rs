//! What a language serves of each file, through `language_serves`: every relation of a file's scope but the engine's, and no
//! relation of the project's scope, which the project source alone serves.
use super::service::language_serves;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::catalog::service::scope_of;
use ryo_relations::domain::catalog::types::RowScope;
use ryo_relations::domain::relations::service::{all, provenance_of};
use ryo_relations::domain::relations::types::{Provenance, Relation};

/// What a row asks of the language's serves.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The relations of the project's scope it serves.
    ProjectScope,
    /// The engine's relations it serves.
    Engine,
    /// The relations of a file's scope, not the engine's, it does not serve.
    FileScopeLeftOut,
}

type Input = Asked;
/// The relations found.
type Value = Vec<Relation>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("file_and_project_sources_never_serve_one_relation"),
        word: Word::Succeeds,
        input: || Asked::ProjectScope,
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaseName("a_language_serves_none_of_the_engine_s_relations"),
        word: Word::Succeeds,
        input: || Asked::Engine,
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaseName("a_language_serves_every_relation_of_a_file_s_scope_but_the_engine_s"),
        word: Word::Succeeds,
        input: || Asked::FileScopeLeftOut,
        expect: Expect::Value(Vec::new()),
    },
];

fn subject(asked: Input) -> Result<Value, Failure> {
    let serves = language_serves(all());
    Ok(match asked {
        Asked::ProjectScope => serves
            .iter()
            .copied()
            .filter(|relation| scope_of(*relation) == RowScope::Project)
            .collect(),
        Asked::Engine => serves
            .iter()
            .copied()
            .filter(|relation| provenance_of(*relation) == Provenance::Engine)
            .collect(),
        Asked::FileScopeLeftOut => all()
            .iter()
            .copied()
            .filter(|relation| {
                scope_of(*relation) == RowScope::File
                    && provenance_of(*relation) != Provenance::Engine
                    && !serves.contains(relation)
            })
            .collect(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
