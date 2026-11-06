//! Independently authored evaluation-order witnesses; no linter supplies these expectations.
use super::service::schedule;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    definitions::service::definition,
    model::types::{BindingAction, BindingSchedule, Declarations},
    syntax::service::{kind, kind_name, name_of},
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};
type Input = &'static str;
type Value = &'static str;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("initializer read precedes its store"),
        word: Word::Succeeds,
        input: || "x = x\n",
        expect: Expect::Value("read:x@module|bind:x@module"),
    },
    Case {
        name: CaseName("defaults are eager and parameters bind at deferred body entry"),
        word: Word::Succeeds,
        input: || "def f(x=later):\n    return x\nlater = 1\n",
        expect: Expect::Value(
            "read:later@module|bind:f@module|bind:later@module|bind:x@function-def|read:x@function-def",
        ),
    },
    Case {
        name: CaseName("nested function reads follow completed enclosing body"),
        word: Word::Succeeds,
        input: || "def outer():\n    def inner():\n        return x\n    x = 1\n    return inner\n",
        expect: Expect::Value(
            "bind:outer@module|bind:inner@function-def|bind:x@function-def|read:inner@function-def|read:x@function-def",
        ),
    },
    Case {
        name: CaseName("generator first iterable is eager while target and element are deferred"),
        word: Word::Succeeds,
        input: || "items = (x for x in values)\n",
        expect: Expect::Value(
            "read:values@module|bind:items@module|bind:x@generator-exp|read:x@generator-exp",
        ),
    },
    Case {
        name: CaseName("annotation-only target does not become an initialized store"),
        word: Word::Succeeds,
        input: || "x: Missing\n",
        expect: Expect::Value("annotation:x@module|read:Missing@name"),
    },
    Case {
        name: CaseName("augmented assignment reads target before value and replacement"),
        word: Word::Succeeds,
        input: || "x += value\n",
        expect: Expect::Value("read:x@module|read:value@module|bind:x@module"),
    },
    Case {
        name: CaseName("handler binding is cleaned after body and before following reads"),
        word: Word::Succeeds,
        input: || "try:\n    action()\nexcept Error as error:\n    use(error)\nafter(error)\n",
        expect: Expect::Value(
            "read:action@module|read:Error@module|bind:error@module|read:use@module|read:error@module|cleanup:error@module|read:after@module|read:error@module",
        ),
    },
    Case {
        name: CaseName("branches retain distinct store and deletion events"),
        word: Word::Succeeds,
        input: || "if condition:\n    x = 1\nelse:\n    del x\nuse(x)\n",
        expect: Expect::Value(
            "read:condition@module|bind:x@module|delete:x@module|read:use@module|read:x@module",
        ),
    },
    Case {
        name: CaseName("embedded lambda defaults stay in the type-expression phase"),
        word: Word::Succeeds,
        input: || "x: \"(lambda exec=exec(code): exec(code))()\"\n",
        expect: Expect::Value(
            "annotation:x@module|read:exec@call|read:code@call|bind:exec@lambda|read:exec@lambda|read:code@lambda",
        ),
    },
];
fn subject(source: Input) -> Result<Value, Failure> {
    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
    use ryo::languages::Parse as _;
    use std::sync::Arc;
    let Ok(mut file) = parsed(source.as_bytes()) else {
        return Err(());
    };
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let parser = ParseContract::new(
        crate::settings::PythonParserSettings::default(),
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
    );
    let mut model = Declarations::default();
    parser.file_model(&tree, &mut model);
    let mut plan = BindingSchedule::default();
    schedule(&tree, &model, &mut file.symbols, &mut plan);
    let mut observed = Vec::new();
    for step in &plan.steps {
        let action = match step.action {
            BindingAction::Read => "read",
            BindingAction::Bind => "bind",
            BindingAction::Delete => "delete",
            BindingAction::Cleanup => "cleanup",
            BindingAction::Annotation => "annotation",
            BindingAction::Global => "global",
            BindingAction::Nonlocal => "nonlocal",
            BindingAction::BeginPhase
            | BindingAction::EndPhase
            | BindingAction::BeginAlternatives
            | BindingAction::NextAlternative
            | BindingAction::EndAlternatives => continue,
        };
        let name = definition(&tree, &model, step.occurrence)
            .map(|defined| defined.name)
            .or_else(|| name_of(&tree, step.occurrence))
            .ok_or(())?;
        let name = languages::text_of(&file.symbols, tree.source, name).ok_or(())?;
        observed.push(format!(
            "{action}:{}@{}",
            name.0,
            kind_name(kind(&tree, step.phase)).0
        ));
    }
    let first = plan.steps.clone();
    schedule(&tree, &model, &mut file.symbols, &mut plan);
    assert_eq!(
        first, plan.steps,
        "repeated scheduling cannot retain or duplicate previous events"
    );
    Ok(Box::leak(observed.join("|").into_boxed_str()))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(CHECKS, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}

/// Compiler-mode metadata is shared with syntax checks, not inferred from alias spelling.
fn future_annotation_timing_uses_validated_module_activation() {
    use crate::domain::model::types::BindingAnnotationTiming;
    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
    use crate::settings::{PythonParserSettings, PythonTargetVersion};
    use ryo::languages::Parse as _;
    use std::sync::Arc;

    let cases = [
        ("from __future__ import annotations\nx: Missing\n", true),
        (
            "\"module docstring\"\nfrom __future__ import annotations\nx: Missing\n",
            true,
        ),
        (
            "from __future__ import division\nfrom __future__ import annotations\nx: Missing\n",
            true,
        ),
        ("from .__future__ import annotations\nx: Missing\n", false),
        (
            "lazy from __future__ import annotations\nx: Missing\n",
            false,
        ),
        (
            "x=1\nfrom __future__ import annotations\nx: Missing\n",
            false,
        ),
        (
            "def outer():\n from __future__ import annotations\nx: Missing\n",
            false,
        ),
        (
            "if condition:\n from __future__ import annotations\nx: Missing\n",
            false,
        ),
        (
            "from __future__ import annotations, not_a_feature\nx: Missing\n",
            false,
        ),
        ("from __future__ import (\n", false),
        ("x: Missing\n", false),
    ];
    for (source, enabled) in cases {
        for target in [PythonTargetVersion::Py313, PythonTargetVersion::Py314] {
            let parser = ParseContract::new(
                PythonParserSettings::new(target),
                Arc::new(ruff_python_parser()),
                Arc::new(postcard()),
            );
            let (mut elements, mut symbols) = (
                languages::Elements::default(),
                languages::Symbols::default(),
            );
            languages::contract::build(
                &parser,
                languages::ParseRequest {
                    path: languages::SourcePath("compiler-mode.py"),
                    file: languages::FileId(0),
                    bytes: ByteSlice(source.as_bytes()),
                    notes: languages::NoteRequest::Unwanted,
                    fields: languages::FieldRequest::Unwanted,
                },
                &mut elements,
                &mut symbols,
            )
            .expect("Python source keeps a parsed or recovered tree");
            let tree = Tree {
                elements: &elements,
                source: ByteSlice(source.as_bytes()),
                file: languages::FileId(0),
            };
            let mut model = Declarations::default();
            parser.file_model(&tree, &mut model);
            let mut plan = BindingSchedule::default();
            schedule(&tree, &model, &mut symbols, &mut plan);
            let expected = if enabled || target >= PythonTargetVersion::Py314 {
                BindingAnnotationTiming::Deferred
            } else {
                BindingAnnotationTiming::Immediate
            };
            assert_eq!(plan.annotations, expected, "{target:?}: {source}");
            let property = languages::property_at(
                &elements,
                languages::Position(0),
                crate::domain::syntax::constants::MODULE_FUTURE_ANNOTATIONS,
            );
            assert_eq!(
                property.map(|value| value.0),
                Some(if enabled { "enabled" } else { "disabled" }),
                "{source}"
            );
        }
    }
}

/// Each check below states its own source and asserts its own outcome.
const CHECKS: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("future_annotation_timing_uses_validated_module_activation"),
        word: Word::Succeeds,
        input: || future_annotation_timing_uses_validated_module_activation,
        expect: Expect::Value(()),
    },
];
