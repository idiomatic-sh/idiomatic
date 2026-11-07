//! The syntax adapter holds its port contract and the independently pinned standard tree.
use super::adapter::SyntaxOxcParserAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::kind_name;
use crate::outbound::syntax::fixtures::contract::{check, elements_of};
#[derive(Clone, Copy)]
enum Given {
    Source(&'static str),
    Corpus,
    ChangedRoot,
    Corrupt(u8),
}
type Input = Given;
type Value = &'static str;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_file_is_its_root_alone"),
        word: Word::Succeeds,
        input: || Given::Source(""),
        expect: Expect::Value("program"),
    },
    Case {
        name: CaseName("the_text_of_a_file_is_a_token_under_its_root"),
        word: Word::Succeeds,
        input: || Given::Source("a"),
        expect: Expect::Value("program expression-statement identifier identifier-token"),
    },
    Case {
        name: CaseName("every_pinned_source_has_the_independent_standard_tree"),
        word: Word::Succeeds,
        input: || Given::Corpus,
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_changed_root_is_a_difference"),
        word: Word::Succeeds,
        input: || Given::ChangedRoot,
        expect: Expect::Value("difference"),
    },
    Case {
        name: CaseName("missing_source_is_broken_evidence"),
        word: Word::Succeeds,
        input: || Given::Corrupt(0),
        expect: Expect::Value("broken"),
    },
    Case {
        name: CaseName("empty_dump_is_broken_evidence"),
        word: Word::Succeeds,
        input: || Given::Corrupt(1),
        expect: Expect::Value("broken"),
    },
    Case {
        name: CaseName("unknown_kind_is_broken_evidence"),
        word: Word::Succeeds,
        input: || Given::Corrupt(2),
        expect: Expect::Value("broken"),
    },
    Case {
        name: CaseName("skipped_parent_is_broken_evidence"),
        word: Word::Succeeds,
        input: || Given::Corrupt(3),
        expect: Expect::Value("broken"),
    },
];
fn subject(input: Input) -> Value {
    match input {
        Given::Source(text) => {
            check(&SyntaxOxcParserAdapter);
            Box::leak(
                elements_of(&SyntaxOxcParserAdapter, text)
                    .iter()
                    .map(|(kind, _)| kind_name(*kind).0)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .into_boxed_str(),
            )
        }
        Given::Corpus => Box::leak(corpus_differences().join("\n").into_boxed_str()),
        Given::ChangedRoot => {
            let standard = super::fixtures::walk::walked(
                "function f<T>(x: T): T { return a + b; }\nclass C extends D implements I {}\n",
            );
            let mut altered = super::fixtures::walk::walked(
                "function f<T>(x: T): T { return a + b; }\nclass C extends D implements I {}\n",
            );
            altered.end.0 += 1;
            if difference(&altered, &standard, "root").is_some() {
                "difference"
            } else {
                "same"
            }
        }
        Given::Corrupt(case) => {
            // A corrupt reference must panic; a tree it gives back would mean it was accepted.
            let outcome = std::panic::catch_unwind(|| match case {
                0 => super::fixtures::walk::reference(
                    "this source is deliberately absent from the reference inventory",
                )
                .ok(),
                1 => Some(super::fixtures::walk::parsed("")),
                2 => Some(super::fixtures::walk::parsed("tree 0 N missing-kind 0 0")),
                _ => Some(super::fixtures::walk::parsed("tree 1 N program 0 0")),
            });
            match outcome {
                Ok(_tree) => "accepted",
                Err(payload) => {
                    drop(payload);
                    "broken"
                }
            }
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

/// The element table as the same typed tree the independent reference supplies.
fn actual(
    elements: &ryo::languages::Elements,
    at: ryo::languages::Position,
) -> ryo::languages::contract::Walked {
    use ryo::languages::{
        self,
        contract::{Leafness, Walked},
    };
    Walked {
        kind: languages::kind_at(elements, at),
        leaf: if languages::flags_at(elements, at).0 & languages::TOKEN.0 == 0 {
            Leafness::Node
        } else {
            Leafness::Token
        },
        start: languages::start_at(elements, at),
        end: languages::end_at(elements, at),
        children: languages::children(elements, at)
            .map(|child| actual(elements, child))
            .collect(),
    }
}

/// The first differing tree element, with its standard names.
fn difference(
    ours: &ryo::languages::contract::Walked,
    theirs: &ryo::languages::contract::Walked,
    path: &str,
) -> Option<String> {
    use crate::domain::syntax::service::kind_of;
    if (ours.kind, ours.leaf, ours.start, ours.end)
        != (theirs.kind, theirs.leaf, theirs.start, theirs.end)
    {
        return Some(format!(
            "{path}: {} {:?}..{:?}, reference {} {:?}..{:?}",
            kind_name(kind_of(ours.kind)).0,
            ours.start,
            ours.end,
            kind_name(kind_of(theirs.kind)).0,
            theirs.start,
            theirs.end
        ));
    }
    if ours.children.len() != theirs.children.len() {
        return Some(format!(
            "{path} {}: children {:?}, reference {:?}",
            kind_name(kind_of(ours.kind)).0,
            ours.children
                .iter()
                .map(|child| kind_name(kind_of(child.kind)).0)
                .collect::<Vec<_>>(),
            theirs
                .children
                .iter()
                .map(|child| kind_name(kind_of(child.kind)).0)
                .collect::<Vec<_>>()
        ));
    }
    ours.children
        .iter()
        .zip(&theirs.children)
        .enumerate()
        .find_map(|(index, (left, right))| difference(left, right, &format!("{path}/{index}")))
}

fn sources(directory: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("fixture directory exists") {
        let path = entry.expect("fixture is readable").path();
        if path.is_dir() {
            sources(&path, found);
            continue;
        }
        if path
            .extension()
            .is_some_and(|extension| extension == "ts" || extension == "tsx")
        {
            found.push(path);
        }
    }
}

fn corpus_differences() -> Vec<String> {
    use super::fixtures::walk::reference;
    use crate::domain::language::service::dialect;
    use crate::outbound::syntax::interface::SyntaxPort as _;
    use crate::outbound::syntax::types::SourceText;
    use ryo::languages;
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let standard = directory.join("standard").join("output.txt");
    assert!(
        standard.is_file(),
        "the independently pinned dump is present"
    );
    let mut files = Vec::new();
    sources(&directory.join("suite"), &mut files);
    sources(&directory.join("relations"), &mut files);
    sources(&directory.join("corpus"), &mut files);
    files.sort();
    files
        .into_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(&path).expect("fixture source is readable");
            let (mut elements, mut symbols) = (
                languages::Elements::default(),
                languages::Symbols::default(),
            );
            let mut builder = languages::table_builder(
                &mut elements,
                &mut symbols,
                languages::ByteSlice(source.as_bytes()),
            );
            SyntaxOxcParserAdapter.read(
                SourceText(source.as_bytes()),
                dialect(languages::SourcePath(
                    path.to_str().expect("fixture path is UTF-8"),
                )),
                &mut builder,
            );
            match reference(&source) {
                Ok(expected) => difference(
                    &actual(&elements, languages::Position(0)),
                    &expected,
                    "root",
                )
                .map(|why| format!("{}: {why}", path.display())),
                Err(languages::contract::ReferenceRefusal)
                    if languages::diagnostic_count(&elements).0 > 0 =>
                {
                    None
                }
                Err(languages::contract::ReferenceRefusal) => Some(format!(
                    "{}: reference refused input but parser reported no error",
                    path.display()
                )),
            }
        })
        .collect()
}
