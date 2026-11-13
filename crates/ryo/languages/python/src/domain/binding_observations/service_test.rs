//! The `binding_observations` service's cases: the events, read states and shadow edges observing the schedule of a parsed module's
//! straight-line statements records in one frame.

use super::service::observe;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::binding_results::{
    service::{event, relationship, state},
    types::ReadContext,
};
use crate::domain::binding_schedule::service::schedule;
use crate::domain::binding_states::service::open;
use crate::domain::model::types::{BindingAnalysisIndex, Declarations};
use crate::domain::syntax::service::written;
use crate::fixtures::reading::parsed;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
use crate::settings::PythonParserSettings;
use ryo::languages::{self, ByteSlice, Integer, Parse as _, Position, Tree};
use std::sync::Arc;

/// A module of straight-line statements.
type Input = &'static str;
/// The events as `kind:text`, then the reads as `text=state`, then the shadows as `binding>previous relationship`, the three lists
/// joined by ` / ` and their items by `|`.
type Value = &'static str;
/// A source the parser refuses.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_read_before_any_store_is_unbound"),
        word: Word::Succeeds,
        input: || "use(x)\n",
        expect: Expect::Value("read:use|read:x / use=unbound|x=unbound / "),
    },
    Case {
        name: CaseName("a_second_store_replaces_the_first_and_the_read_sees_it"),
        word: Word::Succeeds,
        input: || "x = 1\nx = 2\nuse(x)\n",
        expect: Expect::Value(
            "bind:x|bind:x|read:use|read:x / use=unbound|x=unique-declaration / x>x replacement",
        ),
    },
    Case {
        name: CaseName("a_builtin_read_is_a_builtin"),
        word: Word::Succeeds,
        input: || "len\n",
        expect: Expect::Value("read:len / len=builtin / "),
    },
    Case {
        name: CaseName("a_deletion_reads_then_unbinds"),
        word: Word::Succeeds,
        input: || "x = 1\ndel x\nuse(x)\n",
        expect: Expect::Value(
            "bind:x|delete:x|read:use|read:x / x=unique-declaration|use=unbound|x=unbound / ",
        ),
    },
    Case {
        name: CaseName("a_wildcard_import_is_a_wildcard_event"),
        word: Word::Succeeds,
        input: || "from m import *\nuse(y)\n",
        expect: Expect::Value("wildcard:*|read:use|read:y / use=uncertain|y=uncertain / "),
    },
    Case {
        name: CaseName("a_global_statement_records_each_of_its_names"),
        word: Word::Succeeds,
        input: || "global a, b\n",
        expect: Expect::Value("global:a|global:b /  / "),
    },
];

/// Observes every step of the source's schedule in one module frame.
fn subject(source: Input) -> Result<Value, Failure> {
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
    let frame = open(&mut index.environments, None, Position(0));
    let steps = index.schedule.steps.clone();
    for (ordinal, step) in steps.into_iter().enumerate() {
        let ordinal = Integer(i64::try_from(ordinal).map_err(|_| ())?);
        observe(
            &tree,
            &model,
            frame,
            step,
            ReadContext::Value,
            ordinal,
            &mut index,
        );
    }
    let events: Vec<String> = index
        .events
        .iter()
        .map(|each| {
            format!(
                "{}:{}",
                event(each.kind).0,
                written(&tree, each.occurrence).0
            )
        })
        .collect();
    let reads: Vec<String> = index
        .reads
        .iter()
        .map(|each| {
            format!(
                "{}={}",
                written(&tree, each.reference).0,
                state(each.state).0
            )
        })
        .collect();
    let shadows: Vec<String> = index
        .shadows
        .iter()
        .map(|each| {
            format!(
                "{}>{} {}",
                written(&tree, each.binding).0,
                written(&tree, each.previous).0,
                relationship(each.relationship).0
            )
        })
        .collect();
    Ok(Box::leak(
        format!(
            "{} / {} / {}",
            events.join("|"),
            reads.join("|"),
            shadows.join("|")
        )
        .into_boxed_str(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
