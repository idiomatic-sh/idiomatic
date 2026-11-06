//! Canonical typed-reader contract and explicit frontend refusal.
use super::super::fixtures::contract::check;
use super::item::BindingEventItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::collections::relations::scope::item::ScopeItem;
use crate::fixtures::reading::proposed;
use ryo::languages::contract::{Reading, Want};
use std::collections::HashMap;
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty module has complete empty binding facts"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("malformed source does not prove absence"),
        word: Word::Succeeds,
        input: || "if:\n    pass\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["binding analysis requires complete supported syntax"],
        }),
    },
];
#[test]
fn succeeds() {
    check::<BindingEventItem>();
    run(CASES, Word::Succeeds, |input| {
        Ok(proposed::<BindingEventItem>(input))
    });
    run(CHECKS, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}

#[derive(Clone, Copy)]
struct Event {
    name: &'static str,
    scope: u32,
    phase: u32,
    kind: &'static str,
    ordinal: i64,
    line: u32,
}

fn events(source: &'static str) -> Vec<Event> {
    let answer = proposed::<BindingEventItem>(source);
    assert!(
        answer.causes.is_empty(),
        "complete source has binding events"
    );
    answer
        .rows
        .iter()
        .map(|row| {
            let [
                Want::Node(_occurrence),
                Want::Name(name),
                Want::Node(scope),
                Want::Node(phase),
                Want::Name(kind),
                Want::Number(ordinal),
                Want::File(_),
                Want::Span(start, _),
            ] = **row
            else {
                panic!("binding-event row has the canonical columns")
            };
            Event {
                name,
                scope,
                phase,
                kind,
                ordinal,
                line: start.line,
            }
        })
        .collect()
}

fn scope_parents(source: &'static str) -> HashMap<u32, u32> {
    let answer = proposed::<ScopeItem>(source);
    assert!(answer.causes.is_empty(), "complete source has scope rows");
    answer
        .rows
        .iter()
        .map(|row| {
            let [
                Want::Node(parent),
                Want::Node(child),
                Want::File(_),
                Want::Span(_, _),
            ] = **row
            else {
                panic!("scope row has the canonical columns")
            };
            (child, parent)
        })
        .collect()
}

fn event<'a>(events: &'a [Event], name: &str, kind: &str) -> &'a Event {
    let mut found = events
        .iter()
        .filter(|event| event.name == name && event.kind == kind);
    let first = found.next().expect("the requested event is present");
    assert!(found.next().is_none(), "the requested event is unique");
    first
}

fn event_on_line<'a>(events: &'a [Event], name: &str, kind: &str, line: u32) -> &'a Event {
    events
        .iter()
        .find(|event| event.name == name && event.kind == kind && event.line == line)
        .expect("the requested source-line event is present")
}

fn check_site(events: &[Event]) -> &Event {
    events
        .iter()
        .filter(|event| event.name == "x" && event.kind == "read")
        .min_by_key(|event| event.ordinal)
        .expect("the BoolOp has a source-order name read")
}

fn contextual_builtin_lookup_uses_the_check_site_event_order() {
    let source = "x and not x and (lambda list: list())\n";
    let rows = events(source);
    let site = check_site(&rows);
    let parameter = event(&rows, "list", "bind");
    let call_read = rows
        .iter()
        .find(|row| row.name == "list" && row.kind == "read")
        .expect("lambda body reads list");
    assert_eq!(site.phase, 0, "the check-site read is in the module phase");
    assert!(site.ordinal < parameter.ordinal);
    assert!(parameter.ordinal < call_read.ordinal);
    assert_ne!(site.phase, call_read.phase, "lambda body is deferred");
}

fn contextual_lookup_sees_outer_parameters_but_not_later_function_locals() {
    let source = "def f(list):\n    x and not x and (lambda: list())\n";
    let rows = events(source);
    let site = check_site(&rows);
    let parameter = event(&rows, "list", "bind");
    let call_read = event(&rows, "list", "read");
    assert_eq!(parameter.scope, site.scope);
    assert_eq!(parameter.phase, site.phase);
    assert!(parameter.ordinal < site.ordinal);
    assert!(site.ordinal < call_read.ordinal);

    let later_source = "def f():\n    x and not x and (lambda: list())\n    list = custom\n";
    let later_rows = events(later_source);
    let later_site = check_site(&later_rows);
    let later_local = later_rows
        .iter()
        .find(|row| row.name == "list" && row.kind == "bind" && row.line == 3)
        .expect("later function-local binding exists");
    let later_call_read = event(&later_rows, "list", "read");
    assert_eq!(later_local.scope, later_site.scope);
    assert_eq!(later_local.phase, later_site.phase);
    assert!(later_site.ordinal < later_local.ordinal);
    assert!(later_local.ordinal < later_call_read.ordinal);
}

fn deferred_function_body_observes_later_module_binding_events() {
    let source = "def f():\n    x and not x and (lambda: list())\nlist = custom\n";
    let rows = events(source);
    let site = check_site(&rows);
    let module_binding = rows
        .iter()
        .find(|row| row.name == "list" && row.kind == "bind" && row.line == 3)
        .expect("later module binding exists");
    let call_read = event(&rows, "list", "read");
    assert_ne!(module_binding.scope, site.scope);
    assert_eq!(
        module_binding.phase, 0,
        "the assignment is in module execution"
    );
    assert_ne!(
        site.phase, module_binding.phase,
        "the function body is deferred"
    );
    assert!(module_binding.ordinal < site.ordinal);
    assert!(site.ordinal < call_read.ordinal);
}

fn deletion_and_exception_cleanup_precede_the_lookup_anchor() {
    let source = "list = custom\ndel list\nx and not x and (lambda: list())\n";
    let rows = events(source);
    let site = check_site(&rows);
    let binding = rows
        .iter()
        .find(|row| row.name == "list" && row.kind == "bind")
        .expect("the module binding exists");
    let deletion = event(&rows, "list", "delete");
    assert!(binding.ordinal < deletion.ordinal);
    assert!(deletion.ordinal < site.ordinal);

    let exception_source =
        "try:\n    pass\nexcept Exception as list:\n    pass\nx and not x and (lambda: list())\n";
    let exception_rows = events(exception_source);
    let exception_site = check_site(&exception_rows);
    let exception_binding = exception_rows
        .iter()
        .find(|row| row.name == "list" && row.kind == "bind")
        .expect("exception target binding exists");
    let cleanup = event(&exception_rows, "list", "cleanup");
    let exception_call_read = event(&exception_rows, "list", "read");
    assert!(exception_binding.ordinal < cleanup.ordinal);
    assert!(cleanup.ordinal < exception_site.ordinal);
    assert!(exception_site.ordinal < exception_call_read.ordinal);
}

fn class_lookup_uses_the_current_class_and_skips_outer_class_scopes() {
    let source = "class Outer:\n    list = custom\n    x and not x and (lambda list: list())\n";
    let rows = events(source);
    let site = check_site(&rows);
    let class_binding = event_on_line(&rows, "list", "bind", 2);
    let lambda_parameter = rows
        .iter()
        .find(|row| row.name == "list" && row.kind == "bind" && row.ordinal > site.ordinal)
        .expect("deferred lambda parameter binding exists");
    assert_eq!(class_binding.scope, site.scope);
    assert_eq!(class_binding.phase, site.phase);
    assert!(class_binding.ordinal < site.ordinal);
    assert!(site.ordinal < lambda_parameter.ordinal);

    let inner_source = "class Outer:\n    list = custom\n    class Inner:\n        x and not x and (lambda list: list())\n";
    let inner_rows = events(inner_source);
    let inner_site = check_site(&inner_rows);
    let outer_class_binding = inner_rows
        .iter()
        .find(|row| row.name == "list" && row.kind == "bind" && row.line == 2)
        .expect("outer class binding exists");
    let scopes = scope_parents(inner_source);
    assert_ne!(outer_class_binding.scope, inner_site.scope);
    assert_eq!(
        scopes.get(&inner_site.scope),
        Some(&outer_class_binding.scope),
        "the outer class is the physical parent of the inner check scope"
    );
    assert!(outer_class_binding.ordinal < inner_site.ordinal);
}

/// Each check below states its own source and asserts its own outcome.
const CHECKS: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("contextual_builtin_lookup_uses_the_check_site_event_order"),
        word: Word::Succeeds,
        input: || contextual_builtin_lookup_uses_the_check_site_event_order,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("contextual_lookup_sees_outer_parameters_but_not_later_function_locals"),
        word: Word::Succeeds,
        input: || contextual_lookup_sees_outer_parameters_but_not_later_function_locals,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("deferred_function_body_observes_later_module_binding_events"),
        word: Word::Succeeds,
        input: || deferred_function_body_observes_later_module_binding_events,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("deletion_and_exception_cleanup_precede_the_lookup_anchor"),
        word: Word::Succeeds,
        input: || deletion_and_exception_cleanup_precede_the_lookup_anchor,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("class_lookup_uses_the_current_class_and_skips_outer_class_scopes"),
        word: Word::Succeeds,
        input: || class_lookup_uses_the_current_class_and_skips_outer_class_scopes,
        expect: Expect::Value(()),
    },
];
