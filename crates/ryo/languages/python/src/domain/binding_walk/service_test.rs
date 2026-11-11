//! The `binding_walk` service's cases: the explicit binding events a grammar-order walk of a parsed source appends, with the
//! alternatives it brackets and the deferred phases it leaves for later.

use super::service::walk;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::service::definition;
use crate::domain::model::types::{BindingAction, BindingPhaseKind, BindingSchedule, Declarations};
use crate::domain::syntax::service::{kind, kind_name, name_of, written};
use crate::fixtures::reading::parsed;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
use crate::settings::PythonParserSettings;
use ryo::languages::{self, ByteSlice, Parse as _, Position, Tree};
use std::sync::Arc;

/// A source.
type Input = &'static str;
/// Every step as `action:name` (`action:text` for a global or nonlocal statement), or `action@kind` for a bracket, joined by `|`; then each deferred phase as `kind@node`.
type Value = &'static str;
/// A source the parser refuses.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_assignment_reads_its_value_before_it_binds"),
        word: Word::Succeeds,
        input: || "x = y\n",
        expect: Expect::Value("read:y|bind:x"),
    },
    Case {
        name: CaseName("if_branches_are_bracketed_alternatives"),
        word: Word::Succeeds,
        input: || "if c:\n    x = 1\nelse:\n    del x\n",
        expect: Expect::Value("read:c|begin:if|bind:x|next:if|delete:x|end:if"),
    },
    Case {
        name: CaseName("a_loop_body_is_an_alternative_and_its_else_follows"),
        word: Word::Succeeds,
        input: || "for i in xs:\n    pass\nelse:\n    y = 1\n",
        expect: Expect::Value("read:xs|begin:for|bind:i|next:for|end:for|bind:y"),
    },
    Case {
        name: CaseName("a_function_evaluates_defaults_binds_and_defers_its_body"),
        word: Word::Succeeds,
        input: || "def f(a=d):\n    return a\n",
        expect: Expect::Value("read:d|bind:f|callable@function-def"),
    },
    Case {
        name: CaseName("try_handlers_bind_and_clean_up_their_name"),
        word: Word::Succeeds,
        input: || "try:\n    a\nexcept E as e:\n    b\nfinally:\n    c\n",
        expect: Expect::Value(
            "begin:try|read:a|next:try|read:E|bind:e|read:b|cleanup:e|end:try|read:c",
        ),
    },
    Case {
        name: CaseName("bool_operands_after_the_first_are_alternatives"),
        word: Word::Succeeds,
        input: || "a or b\n",
        expect: Expect::Value("read:a|begin:a|read:b|next:a|end:a"),
    },
    Case {
        name: CaseName("an_annotation_only_name_is_an_annotation_event"),
        word: Word::Succeeds,
        input: || "x: int\nglobal g\n",
        expect: Expect::Value("read:int|annotation:x|global:global g"),
    },
];

/// The text of an action.
const fn action(action: BindingAction) -> &'static str {
    match action {
        BindingAction::Read => "read",
        BindingAction::Bind => "bind",
        BindingAction::Delete => "delete",
        BindingAction::Cleanup => "cleanup",
        BindingAction::Annotation => "annotation",
        BindingAction::Global => "global",
        BindingAction::Nonlocal => "nonlocal",
        BindingAction::BeginPhase => "begin-phase",
        BindingAction::EndPhase => "end-phase",
        BindingAction::BeginAlternatives => "begin",
        BindingAction::NextAlternative => "next",
        BindingAction::EndAlternatives => "end",
    }
}

/// Walks the source from its root.
fn subject(source: Input) -> Result<Value, Failure> {
    let file = parsed(source.as_bytes()).map_err(|_| ())?;
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
    let mut plan = BindingSchedule::default();
    walk(&tree, &model, Position(0), Position(0), &mut plan);
    let mut texts: Vec<String> = plan
        .steps
        .iter()
        .map(|step| {
            let name = definition(&tree, &model, step.occurrence)
                .map(|defined| defined.name)
                .or_else(|| name_of(&tree, step.occurrence))
                .and_then(|name| languages::text_of(&file.symbols, tree.source, name));
            match (step.action, name) {
                (BindingAction::Global | BindingAction::Nonlocal, _) => {
                    format!(
                        "{}:{}",
                        action(step.action),
                        written(&tree, step.occurrence).0
                    )
                }
                (_, Some(name)) => format!("{}:{}", action(step.action), name.0),
                (_, None) => format!(
                    "{}@{}",
                    action(step.action),
                    kind_name(kind(&tree, step.occurrence)).0
                ),
            }
        })
        .collect();
    texts.extend(plan.phases.iter().map(|phase| {
        let deferred = match phase.kind {
            BindingPhaseKind::Callable => "callable",
            BindingPhaseKind::Generator => "generator",
            BindingPhaseKind::Type => "type",
        };
        format!("{deferred}@{}", kind_name(kind(&tree, phase.root)).0)
    }));
    Ok(Box::leak(texts.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
