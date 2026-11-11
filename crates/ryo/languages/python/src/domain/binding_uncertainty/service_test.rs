//! The `binding_uncertainty` service's cases: the mutations a parsed source's schedule indexes, and which reads a loop or an exception
//! handler makes uncertain, with the candidates it adds.

use super::service::{prepare, refine};
use super::types::UncertaintyRefinement;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::binding_results::{service::state, types::ReadState};
use crate::domain::binding_schedule::service::schedule;
use crate::domain::model::types::{BindingAction, BindingAnalysisIndex, Declarations};
use crate::domain::syntax::service::{name_of, written};
use crate::fixtures::reading::parsed;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
use crate::settings::PythonParserSettings;
use ryo::languages::{self, ByteSlice, Parse as _, Tree};
use std::sync::Arc;

/// A source and the state its environment gives every read: unbound, or the unique declaration of its first store.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Every read starts unbound, with no candidate.
    Unbound(&'static str),
    /// Every read starts as the unique declaration its name's first store is.
    Unique(&'static str),
}
/// What a row is given.
type Input = Given;
/// The mutations as `action:name`, then each read as `name=state+candidates`, joined by `|`.
type Value = &'static str;
/// A source the parser refuses.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_straight_read_keeps_its_state"),
        word: Word::Succeeds,
        input: || Given::Unbound("use(x)\nx = 1\n"),
        expect: Expect::Value("bind:x|use=unbound+0|x=unbound+0"),
    },
    Case {
        name: CaseName("a_read_in_a_while_loop_sees_a_later_store_of_the_loop"),
        word: Word::Succeeds,
        input: || Given::Unbound("while c:\n    use(x)\n    x = 1\n"),
        expect: Expect::Value("bind:x|c=unbound+0|use=unbound+0|x=uncertain+1"),
    },
    Case {
        name: CaseName("a_for_iterable_is_read_once_and_stays_certain"),
        word: Word::Succeeds,
        input: || Given::Unbound("for i in x:\n    x = 1\n"),
        expect: Expect::Value("bind:i|bind:x|x=unbound+0"),
    },
    Case {
        name: CaseName("a_handler_read_sees_a_store_of_the_protected_body"),
        word: Word::Succeeds,
        input: || Given::Unbound("try:\n    x = 1\nexcept E:\n    use(x)\n"),
        expect: Expect::Value("bind:x|E=unbound+0|use=unbound+0|x=uncertain+1"),
    },
    Case {
        name: CaseName("a_unique_declaration_inside_the_loop_stays_unique"),
        word: Word::Succeeds,
        input: || Given::Unique("while c:\n    x = 1\n    use(x)\n"),
        expect: Expect::Value(
            "bind:x|c=unique-declaration+0|use=unique-declaration+0|x=unique-declaration+1",
        ),
    },
];

/// Indexes the mutations of the source and refines each of its reads.
fn subject(given: Input) -> Result<Value, Failure> {
    let (Given::Unbound(source) | Given::Unique(source)) = given;
    let mut file = parsed(source.as_bytes()).map_err(|_| ())?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let parser = ParseContract::new(
        PythonParserSettings::default(),
        Arc::new(ruff_python_parser(PythonParserSettings::default())),
        Arc::new(postcard()),
    );
    let mut model = Declarations::default();
    parser.file_model(&tree, &mut model);
    let mut index = BindingAnalysisIndex::default();
    schedule(&tree, &model, &mut file.symbols, &mut index.schedule);
    prepare(&tree, &model, &mut index);
    let mut texts: Vec<String> = index
        .mutations
        .iter()
        .map(|mutation| {
            let action = match mutation.action {
                BindingAction::Bind => "bind",
                BindingAction::Delete => "delete",
                BindingAction::Cleanup => "cleanup",
                BindingAction::Read
                | BindingAction::Annotation
                | BindingAction::Global
                | BindingAction::Nonlocal
                | BindingAction::BeginPhase
                | BindingAction::EndPhase
                | BindingAction::BeginAlternatives
                | BindingAction::NextAlternative
                | BindingAction::EndAlternatives => "other",
            };
            format!("{action}:{}", written(&tree, mutation.occurrence).0)
        })
        .collect();
    for step in index
        .schedule
        .steps
        .iter()
        .filter(|step| step.action == BindingAction::Read)
    {
        let Some(name) = name_of(&tree, step.occurrence) else {
            continue;
        };
        let mut candidates = Vec::new();
        let start = match given {
            Given::Unbound(_) => ReadState::Unbound,
            Given::Unique(_) => {
                if let Some(first) = index
                    .mutations
                    .iter()
                    .find(|mutation| mutation.key.name == name)
                {
                    candidates.push(crate::domain::binding_results::types::Candidate {
                        binding: first.occurrence,
                        scope: first.key.scope,
                    });
                }
                ReadState::UniqueDeclaration
            }
        };
        let refined = refine(UncertaintyRefinement {
            tree: &tree,
            model: &model,
            reference: step.occurrence,
            phase: step.phase,
            name,
            state: start,
            mutations: &index.mutations,
            candidates: &mut candidates,
        });
        texts.push(format!(
            "{}={}+{}",
            written(&tree, step.occurrence).0,
            state(refined).0,
            candidates.len()
        ));
    }
    Ok(Box::leak(texts.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
